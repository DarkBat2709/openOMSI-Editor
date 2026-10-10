//! Spline mode of the in-game map editor. Edits stay in World across tile reloads.
//! Existing records keep their index: attached object rows refer to it, not the ID.

use crate::{editor::Action, scene::World};
use glam::{DVec2, DVec3, Vec3};
use hashbrown::{HashMap, HashSet};
use omsi_geometry::SplineCurve;
use omsi_map::{MapSpline, Tile};

pub type Key = ((i32, i32), i64);
pub const OFFICIAL_UPDATES_DISABLED: bool = cfg!(feature = "standalone-editor");

#[derive(Clone, Default)]
pub struct Edits {
    pub originals: HashMap<Key, MapSpline>,
    pub changed: HashMap<Key, MapSpline>,
    pub added: HashMap<Key, MapSpline>,
    pub dirty: bool,
    pub dirty_tiles: HashSet<(i32, i32)>,
    pub terrain_cache: HashMap<(i32, i32), omsi_map::Terrain>,
}

impl Edits {
    pub fn current(&self, key: Key) -> Option<MapSpline> {
        self.added.get(&key).or_else(|| self.changed.get(&key))
            .or_else(|| self.originals.get(&key)).cloned()
    }

    pub fn overlay(&self, key: (i32, i32), tile: &mut Tile) {
        for s in &mut tile.splines {
            if let Some(e) = self.changed.get(&(key, s.id)).or_else(|| self.added.get(&(key, s.id))) {
                *s = e.clone();
            }
        }
        let mut new: Vec<_> = self.added.iter()
            .filter(|((t, id), _)| *t == key && !tile.splines.iter().any(|s| s.id == *id))
            .map(|(_, s)| s.clone()).collect();
        new.sort_by_key(|s| s.id);
        tile.splines.extend(new);
    }

    pub fn for_tile(&self, key: (i32, i32)) -> HashMap<i64, MapSpline> {
        self.changed.iter().chain(self.added.iter())
            .filter(|((t, _), _)| *t == key).map(|((_, id), s)| (*id, s.clone())).collect()
    }
}

#[derive(Default)]
pub struct SplineEditor {
    pub selected: Option<Key>,
    candidates: Vec<Key>,
    next: usize,
    start: Option<(DVec3, String)>,
    drag_offset: Option<DVec3>,
    drag_before: Option<(Key, MapSpline)>,
    undo: Vec<Undo>,
    connection: Option<Connection>,
    fitted_group: Vec<Key>,
    catalog_file: Option<String>,
}

struct Undo {
    splines: Vec<(Key, MapSpline)>,
    terrain: Vec<((i32, i32), omsi_map::Terrain)>,
    terrain_after: Vec<((i32, i32), omsi_map::Terrain)>,
}

impl Undo {
    fn restore_terrain(&self, terrain: &mut HashMap<(i32, i32), omsi_map::Terrain>) {
        for (key, old) in &self.terrain { terrain.insert(*key, old.clone()); }
    }
}

struct TerrainPlan {
    before: Vec<((i32, i32), omsi_map::Terrain)>,
    after: Vec<((i32, i32), omsi_map::Terrain)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End { Start, Finish }

impl End {
    fn point(self, c: &SplineCurve) -> DVec3 {
        match self { Self::Start => c.point_at(0.0), Self::Finish => c.end_point() }
    }
    fn link(self, s: &MapSpline) -> i64 {
        match self { Self::Start => s.prev_id, Self::Finish => s.next_id }
    }
    fn set_link(self, s: &mut MapSpline, id: i64) {
        match self { Self::Start => s.prev_id = id, Self::Finish => s.next_id = id }
    }
    fn heading(self, c: &SplineCurve) -> f64 {
        c.heading_at(if self == Self::Start { 0.0 } else { c.length })
    }
}

struct Connection {
    source: Key,
    target: Option<Key>,
    preview: Option<ConnectionGeometry>,
    error: Option<String>,
    replace_links: bool,
    occupied: bool,
    allow_transition: bool,
}

#[derive(Debug)]
struct ConnectionGeometry {
    source: MapSpline,
    target: MapSpline,
    source_end: End,
    target_end: End,
    gap: f64,
    neighbors: Vec<(Key, MapSpline)>,
}

impl SplineEditor {
    pub fn pick(&mut self, world: &World, eye: DVec3, forward: Vec3) -> Option<i64> {
        world.collect_editor_splines();
        let loaded = world.loaded_tiles();
        let connecting = self.connection_active();
        if connecting { log::info!("spline connection: selecting the target"); }
        // The picking helper owns and releases the cache guard. Preview generation
        // must run after it returns: refresh_connection reads the same mutex.
        let roads: Vec<_> = {
            let edits = world.spline_edits.lock();
            edits.originals.keys().filter(|k| loaded.contains(&k.0))
                .filter_map(|k| edits.current(*k).filter(|s| !s.deleted).map(|s| (*k, s))).collect()
        };
        let edges: HashMap<_, _> = roads.iter().map(|(k, s)| (*k, edge_markers(world, k.0, s, 8))).collect();
        let picked = self.pick_loaded(&world.spline_edits, &loaded, eye, forward, &edges);
        if connecting { log::info!("spline connection: selection {:?}; pick lock released", self.selected); }
        self.refresh_connection(world);
        if connecting { log::info!("spline connection: preview ready={}", self.connection_status().is_some_and(|(_, ready)| ready)); }
        picked
    }

    fn pick_loaded(&mut self, cache: &parking_lot::Mutex<Edits>, loaded: &[(i32, i32)], eye: DVec3, forward: Vec3, edges: &HashMap<Key, Vec<DVec3>>) -> Option<i64> {
        let edits = cache.lock();
        let f = forward.as_dvec3().normalize_or_zero();
        let mut scores = Vec::new();
        for key in edits.originals.keys() {
            if !loaded.contains(&key.0) { continue; }
            let Some(s) = edits.current(*key).filter(|s| !s.deleted) else { continue };
            let c = curve(key.0, &s);
            let count = (s.length / 2.0).ceil().clamp(1.0, 512.0) as usize;
            let mut best = f64::INFINITY;
            for p in (0..=count).map(|n| c.point_at(s.length * n as f64 / count as f64))
                .chain(edges.get(key).into_iter().flatten().copied()) {
                let d = p - eye;
                let along = d.dot(f);
                if !(0.5..400.0).contains(&along) { continue; }
                let off = (d - f * along).length();
                if off < 6.0 { best = best.min(off + along * 0.002); }
            }
            if best.is_finite() { scores.push((best, *key)); }
        }
        scores.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        self.candidates = scores.into_iter().map(|(_, k)| k).take(20).collect();
        self.next = 1;
        self.selected = self.candidates.first().copied();
        self.drag_offset = None;
        self.drag_before = None;
        self.selected.map(|k| k.1)
    }

    pub fn next_pick(&mut self) -> Option<i64> {
        if self.candidates.is_empty() { self.selected = None; return None; }
        self.selected = Some(self.candidates[self.next % self.candidates.len()]);
        self.next += 1;
        self.selected.map(|k| k.1)
    }

    pub fn describe(&self, world: &World) -> String {
        if let Some((message, _)) = self.connection_status() { return message.replace('\n', " · "); }
        if let Some((p, _)) = &self.start {
            return format!("Start {:.2}/{:.2}/{:.2}: point at target and press Shift+G · B cancels", p.x, p.y, p.z);
        }
        let Some((key, s)) = self.selected.and_then(|k| world.spline_edits.lock().current(k).map(|s| (k, s))) else {
            return "No road selected".into();
        };
        if s.deleted { return format!("Spline {} deleted · Ctrl+Z restores it", key.1); }
        let name = s.file.rsplit(['/', '\\']).next().unwrap_or(&s.file);
        format!("Spline {} · {} · Tile ({},{}) · Length {:.2} m · Radius {:.2} m · Gradient {:.2}/{:.2}% · Height {:.2} m",
            key.1, name, key.0.0, key.0.1, s.length, s.radius, s.grad_start, s.grad_end, s.pos[2])
    }

    pub fn markers(&self, world: &World) -> Vec<DVec3> {
        if let Some((p, _)) = &self.start { return vec![*p]; }
        self.selected.and_then(|k| world.spline_edits.lock().current(k).map(|s| (k, s)))
            .filter(|(_, s)| !s.deleted)
            .map(|(k, s)| edge_markers(world, k.0, &s, 8))
            .unwrap_or_default()
    }

    pub fn connection_active(&self) -> bool { self.connection.is_some() }

    pub fn cancel_connection(&mut self) -> bool { self.connection.take().is_some() }

    pub fn connection_can_replace(&self) -> bool {
        self.connection.as_ref().is_some_and(|c| c.occupied && !c.replace_links)
    }

    pub fn replace_connection(&mut self, world: &World) -> String {
        if !self.connection_can_replace() { return "Select two splines with an occupied connection first".into(); }
        world.collect_editor_splines();
        self.connection.as_mut().unwrap().replace_links = true;
        self.refresh_connection(world);
        self.describe(world)
    }

    pub fn transition_enabled(&self) -> bool {
        self.connection.as_ref().is_some_and(|c| c.allow_transition)
    }

    pub fn toggle_transition(&mut self, world: &World) -> String {
        if let Some(c) = self.connection.as_mut() { c.allow_transition = !c.allow_transition; }
        self.refresh_connection(world);
        self.describe(world)
    }

    pub fn connection_status(&self) -> Option<(String, bool)> {
        let connection = self.connection.as_ref()?;
        match (connection.target, &connection.preview, &connection.error) {
            (Some(target), Some(preview), _) => Some((format!(
                "Spline {} ↔ {} · Distance {:.2} m\n{}\nClick Connect or press G/Enter",
                connection.source.1, target.1, preview.gap,
                if connection.replace_links { "Old connections will be replaced · Ctrl+Z to undo" } else { "Both edge points checked · Preview in light blue" }), true)),
            (Some(target), _, Some(error)) => Some((format!("Spline {} ↔ {}\n{error}\n{}", connection.source.1, target.1,
                if self.connection_can_replace() { "Click Replace connection or press Ctrl+G" } else { "Choose another spline or cancel" }), false)),
            _ => Some((format!("Spline {} is marked in blue\nNow click the second spline", connection.source.1), false)),
        }
    }

    /// G starts a selection-only operation. No records change before confirmation.
    pub fn connect_key(&mut self, world: &World) -> String {
        if self.connection_active() {
            self.refresh_connection(world);
            return self.confirm_connection(world);
        }
        let Some(source) = self.selected.filter(|key| world.spline_edits.lock().current(*key).is_some_and(|s| !s.deleted)) else {
            return "Click the first spline, then press G to connect".into();
        };
        self.start = None;
        self.finish_drag();
        self.connection = Some(Connection { source, target: None, preview: None, error: None, replace_links: false, occupied: false, allow_transition: false });
        self.describe(world)
    }

    pub fn refresh_connection(&mut self, world: &World) {
        let Some(source) = self.connection.as_ref().map(|c| c.source) else { return };
        let target = self.selected.filter(|key| *key != source);
        let connection = self.connection.as_mut().unwrap();
        if connection.target != target { connection.replace_links = false; connection.allow_transition = false; }
        let replace_links = connection.replace_links;
        let allow_transition = connection.allow_transition;
        let loaded = world.loaded_tiles();
        // Building the whole-map index can also read editor state: never do it
        // while holding the spline cache. It distinguishes missing from unloaded IDs.
        let index = replace_links.then(|| world.index());
        let mut occupied = false;
        let result = target.map(|target| {
            let mut preview = {
                let edits = world.spline_edits.lock();
                let (Some(a), Some(b)) = (edits.current(source), edits.current(target)) else { return Err("Spline no longer available".into()) };
                let (x, y, _) = closest_ends(source.0, &a, target.0, &b);
                occupied = (x.link(&a) != 0 && x.link(&a) != b.id) || (y.link(&b) != 0 && y.link(&b) != a.id);
                connection_plan(&edits, &loaded, index.as_deref(), source, target, replace_links, allow_transition)?
            };
            let original = world.spline_edits.lock().current(source).ok_or("Spline no longer available")?;
            let source_type = world.spline_type(&original.file).ok_or("Source profile not found")?;
            let target_type = world.spline_type(&preview.target.file).ok_or("Target profile not found")?;
            align_connection_edges(source.0, &original, &source_type.def, target.0, &target_type.def, &mut preview, allow_transition)?;
            for tile in connection_tiles(source, target, &preview.source) {
                // The renderer already supports roads over tile edges. Keep their record
                // and attachment index in the original tile, and rebuild both sides.
                if (tile.0 - source.0.0).abs() > 1 || (tile.1 - source.0.1).abs() > 1 {
                    return Err("Connection too long; choose closer spline ends".into());
                }
                if world.tile_source(tile.0, tile.1).is_none() {
                    return Err("Connection leaves the map".into());
                }
                if !loaded.contains(&tile) { return Err("Load both splines and the space between them first".into()); }
            }
            Ok(preview)
        });
        let connection = self.connection.as_mut().unwrap();
        // A replacement choice applies only to the exact pair shown to the user.
        // Changing the target starts with the normal occupied-end check again.
        connection.target = target;
        connection.occupied = occupied;
        connection.preview = None;
        connection.error = None;
        match result {
            Some(Ok(preview)) => connection.preview = Some(preview),
            Some(Err(error)) => connection.error = Some(error),
            None => {}
        }
    }

    pub fn confirm_connection(&mut self, world: &World) -> String {
        self.refresh_connection(world);
        let Some(mut connection) = self.connection.take() else { return "Press G on the first spline first".into() };
        let (Some(target), Some(preview)) = (connection.target, connection.preview.take()) else {
            let message = connection.error.clone().unwrap_or_else(|| "Click second spline, then choose Connect".into());
            self.connection = Some(connection);
            return message;
        };
        let source = connection.source;
        let before = {
            let edits = world.spline_edits.lock();
            let (Some(a), Some(b)) = (edits.current(source), edits.current(target)) else {
                return "Spline no longer available; connection cancelled".into();
            };
            let mut before = vec![(source, a), (target, b)];
            for (key, _) in &preview.neighbors {
                let Some(old) = edits.current(*key) else { return "Old neighbour no longer available; connection cancelled".into() };
                before.push((*key, old));
            }
            before
        };
        if before[0].1 == preview.source && before[1].1 == preview.target && preview.neighbors.is_empty() {
            return "These spline ends are already connected".into();
        }
        self.remember(before);
        let tiles = connection_tiles(source, target, &preview.source);
        let mut edits = world.spline_edits.lock();
        for (key, spline) in [(source, preview.source), (target, preview.target)].into_iter().chain(preview.neighbors) {
            if edits.added.contains_key(&key) { edits.added.insert(key, spline); }
            else { edits.changed.insert(key, spline); }
            edits.dirty_tiles.insert(key.0);
        }
        edits.dirty_tiles.extend(tiles);
        edits.dirty = true;
        self.selected = Some(source);
        format!("Splines {} and {} connected · Ctrl+Z to undo · Ctrl+S to save", source.1, target.1)
    }

    pub fn connection_markers(&self, world: &World) -> Vec<(DVec3, [f32; 3], f32)> {
        let Some(connection) = &self.connection else { return Vec::new() };
        let blue = [0.1, 0.4, 1.0];
        let roads: Vec<_> = {
            let edits = world.spline_edits.lock();
            [Some(connection.source), connection.target].into_iter().flatten()
                .filter_map(|key| edits.current(key).filter(|s| !s.deleted).map(|s| (key, s))).collect()
        };
        let mut markers = Vec::new();
        for (key, spline) in roads {
            let color = if key != connection.source && connection.error.is_some() { [1.0, 0.25, 0.1] } else { blue };
            markers.extend(edge_markers(world, key.0, &spline, 8).into_iter().map(|p| (p, color, 0.35)));
        }
        if let (Some(target), Some(preview)) = (connection.target, &connection.preview) {
            markers.extend(edge_markers(world, connection.source.0, &preview.source, 16).into_iter()
                .map(|p| (p, [0.1, 0.85, 1.0], 0.2)));
            for (tile, spline, end) in [(connection.source.0, &preview.source, preview.source_end), (target.0, &preview.target, preview.target_end)] {
                if let Some(ty) = world.spline_type(&spline.file) {
                    markers.extend(edge_points(tile, spline, &ty.def, end).into_iter().map(|p| (p, blue, 0.55)));
                }
            }
        }
        markers
    }

    fn set(&mut self, world: &World, key: Key, s: MapSpline) -> Option<String> {
        if !s.deleted && !inside_tile(&s) { return Some("This edit crosses the tile boundary. G connects ends; Shift+G creates a separate road".into()); }
        let old = world.spline_edits.lock().current(key)?;
        if old == s { return Some(self.describe(world)); }
        if self.drag_offset.is_none() { self.remember(vec![(key, old)]); }
        let mut e = world.spline_edits.lock();
        if e.added.contains_key(&key) { e.added.insert(key, s); }
        else { e.changed.insert(key, s); }
        e.dirty = true;
        e.dirty_tiles.insert(key.0);
        drop(e);
        Some(self.describe(world))
    }

    pub fn apply(&mut self, world: &World, action: &Action) -> Option<String> {
        if self.connection_active() && matches!(action, Action::Straight | Action::SplineUndo) {
            self.cancel_connection();
            return Some("Connection cancelled; nothing changed".into());
        }
        if self.connection_active() { return Some("Choose Connect or Cancel first".into()); }
        if matches!(action, Action::SplineUndo) { return Some(self.undo(world)); }
        if matches!(action, Action::Straight) && self.start.take().is_some() {
            return Some("Spline creation cancelled".into());
        }
        let key = self.selected?;
        let mut s = world.spline_edits.lock().current(key)?;
        match action {
            Action::Copy => {
                let c = curve(key.0, &s);
                let end = c.end_point();
                let heading = c.heading_at(s.length);
                let length = s.length.clamp(1.0, 30.0);
                let goal = end + SplineCurve::dir(heading).extend(s.grad_end / 100.0) * length;
                return Some(self.add(world, &s.file, end, goal, s.tex_offset + s.length));
            }
            Action::Move(d) => { s.pos[0] += d.x; s.pos[1] += d.y; s.pos[2] += d.z; }
            Action::Turn(t) => s.heading = (s.heading + t).rem_euclid(360.0),
            Action::Length(d) => {
                let old = s.length;
                s.length = (s.length + d).clamp(0.5, 500.0);
                for t in s.profile_transitions.iter_mut().flatten() { t.station *= s.length / old; t.span *= s.length / old; }
                if let Some(h) = s.delta_h.as_mut() { *h *= s.length / old.max(0.5); }
            }
            Action::Curvature(d) => {
                let k = if s.radius == 0.0 { 0.0 } else { 1.0 / s.radius };
                let k = (k + d).clamp(-0.05, 0.05);
                s.radius = if k.abs() < 1e-9 { 0.0 } else { 1.0 / k };
            }
            Action::Straight => s.radius = 0.0,
            Action::Grade(d) => {
                s.grad_start = (s.grad_start + d).clamp(-20.0, 20.0);
                s.grad_end = s.grad_start;
                s.delta_h = Some(s.length * s.grad_start / 100.0);
                s.is_h = true;
            }
            Action::Undo => {
                s = world.spline_edits.lock().originals.get(&key)?.clone();
            }
            Action::Variant => {
                let (dir, _) = s.file.rsplit_once(['/', '\\'])?;
                let folder = omsi_cfg::resolve_path(&world.root, dir);
                let mut files: Vec<String> = omsi_cfg::vfs::list_dir(&folder)?.into_iter()
                    .filter(|(p, is_dir)| !is_dir && p.to_string_lossy().to_ascii_lowercase().ends_with(".sli"))
                    .map(|(p, _)| format!("{dir}\\{}", p.to_string_lossy())).collect();
                files.sort();
                if files.is_empty() { return Some("No spline types in this folder".into()); }
                let i = files.iter().position(|f| f.eq_ignore_ascii_case(&s.file)).unwrap_or(0);
                let next = (1..=files.len()).map(|n| &files[(i + n) % files.len()])
                    .find(|f| world.spline_type(f).is_some_and(|t| !t.def.only_editor && !t.def.profiles.is_empty()));
                if s.profile_transitions.iter().any(Option::is_some) {
                    return Some("Profile change: undo the transition connection first".into());
                }
                s.file = next?.clone();
            }
            Action::Delete => {
                s.deleted = true;
                self.start = None;
                self.finish_drag();
                self.candidates.retain(|k| *k != key);
            }
            _ => return None,
        }
        self.set(world, key, s)
    }

    fn remember(&mut self, before: Vec<(Key, MapSpline)>) {
        self.remember_with_terrain(before, Vec::new());
    }

    fn remember_with_terrain(&mut self, splines: Vec<(Key, MapSpline)>, terrain: Vec<((i32, i32), omsi_map::Terrain)>) {
        if self.undo.len() >= 100 { self.undo.remove(0); }
        self.undo.push(Undo { splines, terrain, terrain_after: Vec::new() });
    }

    pub(crate) fn can_undo(&self)->bool{!self.undo.is_empty()}

    pub fn undo(&mut self, world: &World) -> String {
        if self.cancel_connection() { return "Connection cancelled; nothing changed".into(); }
        self.start = None;
        if let Some(before) = self.undo.last() {
            let terrain = world.terrain_edits.lock();
            if before.terrain_after.iter().any(|(key,t)|terrain.get(key)!=Some(t)) {
                return "Undo the newer terrain brush change in terrain mode first".into();
            }
        }
        let Some(before) = self.undo.pop() else { return "No action to undo".into() };
        let selection = before.splines.first().map(|(key, _)| *key).or(self.selected);
        self.drag_offset = None;
        let terrain_keys: Vec<_> = before.terrain.iter().map(|(key, _)| *key).collect();
        {
            let mut terrain = world.terrain_edits.lock();
            before.restore_terrain(&mut terrain);
        }
        let mut edits = world.spline_edits.lock();
        for key in terrain_keys { edits.terrain_cache.remove(&key); edits.dirty_tiles.insert(key); }
        for (key, s) in before.splines {
            if edits.added.contains_key(&key) { edits.added.insert(key, s); }
            else { edits.changed.insert(key, s); }
            edits.dirty_tiles.insert(key.0);
        }
        edits.dirty = true;
        self.selected = selection;
        drop(edits);
        self.describe(world)
    }

    pub fn split(&mut self, world: &World, at: Option<DVec3>) -> String {
        let Some(at) = at else { return "Point at the split location, then press F7".into() };
        let Some((key, s)) = self.selected.and_then(|k| world.spline_edits.lock().current(k).filter(|s| !s.deleted).map(|s| (k, s))) else {
            return "Select a spline first".into();
        };
        let c = curve(key.0, &s);
        let distance = nearest_station(&c, at);
        if distance < 0.5 || s.length - distance < 0.5 || (c.point_at(distance).truncate() - at.truncate()).length() > 15.0 {
            return "Split location must lie on the spline, at least 0.5 m from its ends".into();
        }
        let Some(id) = world.allocate_editor_id() else { return "No free map ID".into() };
        let mut first = curve_part(&s, 0.0, distance);
        let mut second = curve_part(&s, distance, s.length - distance);
        second.id = id;
        first.next_id = id;
        second.prev_id = s.id;
        let mut neighbor = None;
        if s.next_id != 0 {
            world.collect_editor_splines();
            let edits = world.spline_edits.lock();
            for k in edits.originals.keys().filter(|k| k.1 == s.next_id) {
                if let Some(p) = edits.current(*k).filter(|p| !p.deleted && p.prev_id == s.id) {
                    if (curve(k.0, &p).point_at(0.0) - c.end_point()).length() < 2.0 { neighbor = Some((*k, p)); break; }
                }
            }
            if neighbor.is_none() { return "Load neighbouring spline first; nothing changed".into(); }
        }
        let mut deleted = second.clone(); deleted.deleted = true;
        let mut before = vec![(key, s), ((key.0, id), deleted)];
        if let Some((k, p)) = &neighbor { before.push((*k, p.clone())); }
        self.remember(before);
        let mut edits = world.spline_edits.lock();
        if edits.added.contains_key(&key) { edits.added.insert(key, first); }
        else { edits.changed.insert(key, first); }
        edits.originals.insert((key.0, id), second.clone());
        edits.added.insert((key.0, id), second);
        if let Some((k, mut p)) = neighbor {
            p.prev_id = id;
            if edits.added.contains_key(&k) { edits.added.insert(k, p); }
            else { edits.changed.insert(k, p); }
            edits.dirty_tiles.insert(k.0);
        }
        edits.dirty = true;
        edits.dirty_tiles.insert(key.0);
        "Spline split; shape and height kept · Ctrl+Z undoes both parts".into()
    }

    /// Fit short, connected cubic sections to the unmodified terrain. Horizontal
    /// curves, texture distance, attachment indices and the original first ID survive.
    pub fn fit_terrain(&mut self, world: &World) -> String {
        let Some(key) = self.selected else { return "Select a spline first".into() };
        let Some(s) = world.spline_edits.lock().current(key).filter(|s| !s.deleted) else {
            return "Select a spline first".into();
        };
        let c = curve(key.0, &s);
        let count = (s.length / 5.0).ceil().clamp(1.0, 256.0) as usize;
        let Some(ty) = world.spline_type(&s.file) else { return "Road profile not found".into(); };
        let o = origin(key.0).extend(0.0);
        let mut profile = Roadbed::default();
        profile.add_mesh(&omsi_geometry::build_spline_mesh(&ty.def, &c, s.mirror, o), o);
        if profile.faces.is_empty() { return "Road profile has no visible surface".into(); }
        let mut heights = Vec::new();
        for i in 0..=count {
            let p = c.point_at(s.length * i as f64 / count as f64);
            let Some(h) = world.editor_terrain_height(p.x, p.y) else {
                return "Terrain not fully loaded; nothing changed".into();
            };
            heights.push(fitted_height(h, p, &profile));
        }
        let mut pieces = terrain_parts(&s, &heights);
        for part in pieces.iter_mut().skip(1) {
            let Some(id) = world.allocate_editor_id() else { return "No free map ID".into() };
            part.id = id;
        }
        for i in 0..pieces.len() {
            pieces[i].prev_id = if i == 0 { s.prev_id } else { pieces[i - 1].id };
            pieces[i].next_id = if i + 1 == pieces.len() { s.next_id } else { pieces[i + 1].id };
        }
        if pieces.iter().any(|p| !inside_tile(p)) {
            return "Section leaves its tile; create shorter sections first".into();
        }
        let roadbed = match self.prepare_terrain(world, &pieces.iter().map(|s| (key.0, s.clone())).collect::<Vec<_>>(), 2.0) {
            Ok(plan) => plan,
            Err(error) => return error,
        };
        // Repair the next section's reverse link when the old section was subdivided.
        let mut neighbor = None;
        if s.next_id != 0 && pieces.len() > 1 {
            world.collect_editor_splines();
            let edits = world.spline_edits.lock();
            for k in edits.originals.keys().filter(|k| k.1 == s.next_id) {
                if let Some(next) = edits.current(*k).filter(|p| !p.deleted && p.prev_id == s.id) {
                    if (curve(k.0, &next).point_at(0.0) - c.end_point()).length() < 2.0 {
                        neighbor = Some((*k, next)); break;
                    }
                }
            }
            if neighbor.is_none() { return "Load neighbouring spline first; nothing changed".into(); }
        }
        let mut before = vec![(key, s)];
        for part in pieces.iter().skip(1) {
            let mut deleted = part.clone(); deleted.deleted = true;
            before.push(((key.0, part.id), deleted));
        }
        if let Some((k, p)) = &neighbor { before.push((*k, p.clone())); }
        self.remember_with_terrain(before, roadbed.before);
        self.fitted_group = pieces.iter().map(|part| (key.0, part.id)).collect();
        let mut edits = world.spline_edits.lock();
        for (i, part) in pieces.iter().enumerate() {
            let k = (key.0, part.id);
            if i > 0 || edits.added.contains_key(&k) {
                edits.originals.entry(k).or_insert_with(|| part.clone());
                edits.added.insert(k, part.clone());
            } else { edits.changed.insert(k, part.clone()); }
        }
        if let Some((k, mut p)) = neighbor {
            p.prev_id = pieces.last().unwrap().id;
            if edits.added.contains_key(&k) { edits.added.insert(k, p); }
            else { edits.changed.insert(k, p); }
            edits.dirty_tiles.insert(k.0);
        }
        edits.dirty = true;
        edits.dirty_tiles.insert(key.0);
        drop(edits);
        self.commit_terrain(world, roadbed.after);
        format!("Height and road edges adjusted: {} sections · F8 smooths shoulder, Shift+F8 wider · Ctrl+Z to undo", pieces.len())
    }

    /// Shape the terrain under the actual rendered surface, including its profile,
    /// cant, mirror and skew. Terrain fitting creates a group so smoothing also covers its subdivisions.
    pub fn smooth_road(&mut self, world: &World, wide: bool) -> String {
        let Some(selected) = self.selected else { return "Select a road first, then press F8 or click Smooth shoulder".into() };
        let keys = if self.fitted_group.contains(&selected) { self.fitted_group.clone() } else { vec![selected] };
        let roads: Vec<_> = {
            let edits = world.spline_edits.lock();
            keys.iter().filter_map(|key| edits.current(*key).filter(|s| !s.deleted).map(|s| (key.0, s))).collect()
        };
        if roads.is_empty() { return "Select an existing road first".into(); }
        let shoulder = if wide { 6.0 } else { 2.0 };
        let plan = match self.prepare_terrain(world, &roads, shoulder) { Ok(plan) => plan, Err(error) => return error };
        if plan.after.is_empty() { return "Road edges are already adjusted".into(); }
        let count = plan.after.len();
        self.remember_with_terrain(Vec::new(), plan.before);
        self.commit_terrain(world, plan.after);
        format!("Road shoulder smoothed: {} section(s), {shoulder:.0} m blend, {count} tile(s) · Ctrl+Z to undo · Ctrl+S to save", roads.len())
    }

    fn prepare_terrain(&self, world: &World, roads: &[((i32, i32), MapSpline)], shoulder: f64) -> Result<TerrainPlan, String> {
        let mut bed = Roadbed::default();
        for (tile, road) in roads {
            let ty = world.spline_type(&road.file).ok_or_else(|| format!("Road profile not found: {}", road.file))?;
            let o = origin(*tile).extend(0.0);
            let mesh = omsi_geometry::build_spline_mesh(&ty.def, &curve(*tile, road), road.mirror, o);
            bed.add_mesh(&mesh, o);
        }
        if bed.faces.is_empty() { return Err("Road profile has no surface to smooth".into()); }
        let loaded: HashMap<_, _> = world.loaded_tiles().into_iter().filter_map(|key| world.editor_terrain_tile(key).map(|t| (key, t))).collect();
        let guard = loaded.values().map(|t| omsi_map::tile_size() / t.cells.max(1) as f64).fold(0.0f64, f64::max) * std::f64::consts::SQRT_2;
        let reach = guard + shoulder;
        let bounds = bed.bounds();
        let size = omsi_map::tile_size();
        let mut terrain = HashMap::new();
        for tx in ((bounds[0] - reach) / size).floor() as i32..=((bounds[2] + reach) / size).floor() as i32 {
            for ty in ((bounds[1] - reach) / size).floor() as i32..=((bounds[3] + reach) / size).floor() as i32 {
                let key = (tx, ty);
                if !bed.touches_tile(key, reach) || world.tile_source(tx, ty).is_none() { continue; }
                let t = loaded.get(&key).ok_or_else(|| format!("Load road and edge tile ({tx},{ty}) first; nothing changed"))?;
                if t.cells == 0 || t.heights.len() != t.samples() * t.samples() || t.heights.iter().any(|h| !h.is_finite()) {
                    return Err(format!("Terrain of tile ({tx},{ty}) contains invalid heights; nothing changed"));
                }
                terrain.insert(key, t.clone());
            }
        }
        if terrain.is_empty() { return Err("Load terrain first; nothing changed".into()); }
        Ok(terrain_plan(&bed, &terrain, guard, shoulder))
    }

    fn commit_terrain(&mut self, world: &World, after: Vec<((i32, i32), omsi_map::Terrain)>) {
        if let Some(undo) = self.undo.last_mut() { undo.terrain_after = after.clone(); }
        let keys: Vec<_> = after.iter().map(|(key, _)| *key).collect();
        {
            let mut terrain = world.terrain_edits.lock();
            for (key, t) in after { terrain.insert(key, t); }
        }
        let mut edits = world.spline_edits.lock();
        for key in keys { edits.terrain_cache.remove(&key); edits.dirty_tiles.insert(key); }
        edits.dirty = true;
    }

    /// Branch from the selected curved road at the point the camera aims at. Use its
    /// actual profile edge and local tangent, rather than the road's start heading.
    pub fn branch(&mut self, world: &World, at: Option<DVec3>, side: f64) -> String {
        let Some(at) = at else { return "Point at the connection location, then press R".into() };
        let Some((key, s)) = self.selected.and_then(|k| world.spline_edits.lock().current(k).filter(|s| !s.deleted).map(|s| (k, s))) else {
            return "Select the existing road first".into();
        };
        let c = curve(key.0, &s);
        let station = nearest_station(&c, at);
        if (c.point_at(station).truncate() - at.truncate()).length() > 15.0 {
            return "Point at a location on the selected road".into();
        }
        let Some(ty) = world.spline_type(&s.file) else { return "Road type not found".into() };
        let (left, right) = profile_edges(&ty.def);
        let edge = if side < 0.0 { left } else { right };
        let start = c.offset_point(station, edge.0, edge.1);
        let heading = c.heading_at(station) + side * 90.0;
        let mut end = start + SplineCurve::dir(heading).extend(0.0) * 20.0;
        end.z = world.editor_terrain_height(end.x, end.y).map(|h| h + 0.05).unwrap_or(start.z);
        let result = self.add(world, &s.file, start, end, 0.0);
        if result.starts_with("New spline") {
            format!("{result} · Side connection; place a junction object for a T-junction with AI")
        } else { result }
    }

    pub fn junction_pose(&self, world: &World, at: DVec3) -> (DVec3, f64) {
        if let Some((key, s)) = self.selected.and_then(|k| world.spline_edits.lock().current(k).filter(|s| !s.deleted).map(|s| (k, s))) {
            let c = curve(key.0, &s);
            let station = nearest_station(&c, at);
            let p = c.point_at(station);
            if (p.truncate() - at.truncate()).length() < 15.0 { return (p, c.heading_at(station)); }
        }
        (at, 0.0)
    }

    /// Snap the free end to another road's endpoint or profile edge. A circular arc
    /// preserves the start tangent; it is a geometry tool, not an AI junction builder.
    pub fn snap_end(&mut self, world: &World, at: Option<DVec3>) -> String {
        let Some(at) = at else { return "Point at the target road, then press H".into() };
        let Some((key, s)) = self.selected.and_then(|k| world.spline_edits.lock().current(k).filter(|s| !s.deleted).map(|s| (k, s))) else {
            return "Select the spline to connect first".into();
        };
        world.collect_editor_splines();
        let loaded = world.loaded_tiles();
        let candidates: Vec<_> = {
            let e = world.spline_edits.lock();
            e.originals.keys().filter(|k| **k != key && loaded.contains(&k.0))
                .filter_map(|k| e.current(*k).filter(|s| !s.deleted).map(|s| (*k, s))).collect()
        };
        let mut best: Option<(f64, DVec3)> = None;
        for (k, other) in candidates {
            let c = curve(k.0, &other);
            let station = nearest_station(&c, at);
            let endpoint = if station < 1.0 { Some(c.point_at(0.0)) }
                else if other.length - station < 1.0 { Some(c.end_point()) } else { None };
            let target = if let Some(p) = endpoint { p } else {
                let Some(ty) = world.spline_type(&other.file) else { continue };
                let (left, right) = profile_edges(&ty.def);
                let dir = SplineCurve::dir(c.heading_at(station));
                let sign = (at.truncate() - c.point_at(station).truncate()).dot(DVec2::new(dir.y, -dir.x));
                let edge = if sign < 0.0 { left } else { right };
                c.offset_point(station, edge.0, edge.1)
            };
            let distance = (target.truncate() - at.truncate()).length();
            if distance < 12.0 && best.as_ref().is_none_or(|(d, _)| distance < *d) { best = Some((distance, target)); }
        }
        let Some((_, target)) = best else { return "No other road found at the target".into() };
        match arc_to(&s, key.0, target) {
            Ok(part) => self.set(world, key, part).unwrap_or_default(),
            Err(e) => e,
        }
    }

    pub fn generate(&mut self, world: &World, at: Option<DVec3>) -> String {
        self.cancel_connection();
        self.finish_drag();
        let Some(mut at) = at else { return "Point at the terrain and press Shift+G".into() };
        at.z = world.editor_terrain_height(at.x, at.y).unwrap_or(at.z) + 0.05;
        if let Some((start, file)) = self.start.clone() {
            let msg = self.add(world, &file, start, at, 0.0);
            if msg.starts_with("New spline") { self.start = None; }
            return msg;
        }
        let file = self.selected.and_then(|k| world.spline_edits.lock().current(k)).map(|s| s.file).or_else(|| self.catalog_file.clone());
        let Some(file) = file else { return "P opens the road catalogue; alternatively select an existing spline".into(); };
        self.start = Some((at, file));
        self.describe(world)
    }

    pub fn choose_file(&mut self, file: String) {
        self.cancel_connection(); self.finish_drag(); self.start = None;
        self.catalog_file = Some(file); self.selected = None;
    }
    pub fn generation_started(&self) -> bool { self.start.is_some() }
    pub fn cancel_generation(&mut self) { self.start = None; }

    pub(crate) fn junction_plan(&self,world:&World,key:Key,id:i64,port:crate::junction_connections::Port,def:&omsi_scenery::Spline)->Result<MapSpline,String>{
        let mut original=world.spline_edits.lock().current(key).ok_or("Road no longer loaded")?;
        // Only a uniquely resolved, explicitly deleted spline may release this end.
        // Missing IDs may belong to live objects or unloaded neighbours.
        let c=curve(key.0,&original);let end=junction_end(&original,&c,id,port.point);
        let link=end.link(&original);
        if link!=0&&link!=id{
            let edits=world.spline_edits.lock();let keys:HashSet<_>=edits.originals.keys().chain(edits.changed.keys()).chain(edits.added.keys()).filter(|k|k.1==link).copied().collect();
            let matches:Vec<_>=keys.into_iter().filter_map(|k|edits.current(k)).collect();
            if deleted_junction_neighbor(link,id,&matches){end.set_link(&mut original,0);log::info!("junction connection: spline {} releases deleted neighbour {}",key.1,link);}
        }
        let ty=world.spline_type(&original.file).ok_or("Road profile missing")?;
        let fitted=junction_fit_port(key.0,&original,&ty.def,id,port,&def)?;
        for tile in connection_tiles(key,key,&fitted){if world.tile_source(tile.0,tile.1).is_none(){return Err("Connection leaves the map".into());}}
        Ok(fitted)
    }
    pub(crate) fn junction_preview(world:&World,parts:&[(Key,MapSpline)])->Vec<[DVec3;2]>{
        let mut points=Vec::new();
        for(key,s)in parts{if let Some(ty)=world.spline_type(&s.file){let edges=physical_edges(&ty.def,s.mirror);let c=curve(key.0,s).with_sli(&ty.def);let n=(s.length/2.0).ceil().max(1.0)as usize;
            for i in 0..n{let a=edges.map(|(x,z)|omsi_geometry::spline_profile_point(&ty.def,&c,false,s.length*i as f64/n as f64,x,z));let b=edges.map(|(x,z)|omsi_geometry::spline_profile_point(&ty.def,&c,false,s.length*(i+1)as f64/n as f64,x,z));points.extend([a,b]);}
        }}points
    }
    pub(crate) fn apply_junction_roads(&mut self,world:&World,pieces:Vec<(Key,MapSpline)>){
        if pieces.is_empty(){return;}
        let mut edits=world.spline_edits.lock();
        for(k,s)in pieces{if edits.added.contains_key(&k){edits.added.insert(k,s);}else{edits.changed.insert(k,s);}edits.dirty_tiles.insert(k.0);}edits.dirty=true;
    }

    pub(crate) fn replace_traffic(&mut self,world:&World,changes:Vec<(Key,MapSpline,MapSpline)>)->Result<usize,String> {
        if changes.is_empty()||changes.len()>500{return Err("Ungültiger KI-Ersetzungsbereich".into());}
        let mut seen=HashSet::new();
        // Validate the complete transaction before touching any map record.
        for (key,before,after) in &changes {
            if !seen.insert(*key)||before.deleted||after.deleted||after.id!=key.1||!valid_spline(after)
                ||crate::traffic_editor::source_of_generated_path(&before.file).is_none()
                ||crate::traffic_editor::source_of_generated_path(&before.file)!=crate::traffic_editor::source_of_generated_path(&after.file) {
                return Err("Ungültige KI-Ersetzung; Karte unverändert".into());
            }
            let (mut tile,_)=world.editor_row_source(key.0)?;world.spline_edits.lock().overlay(key.0,&mut tile);
            if tile.splines.iter().find(|s|s.id==key.1)!=Some(before){return Err("KI-Pfad wurde zwischenzeitlich geändert; Vorschau aktualisieren".into());}
        }
        self.remember(changes.iter().map(|(k,b,_)|(*k,b.clone())).collect());
        let count=changes.len();let mut edits=world.spline_edits.lock();
        for (key,before,after) in changes {
            edits.originals.entry(key).or_insert(before);
            if edits.added.contains_key(&key){edits.added.insert(key,after);}else{edits.changed.insert(key,after);}
            edits.dirty_tiles.insert(key.0);
        }
        edits.dirty=true;Ok(count)
    }

    pub(crate) fn apply_sidewalk(&mut self,world:&World,mut pieces:Vec<((i32,i32),MapSpline)>,existing:Option<Key>,detach:bool)->Result<usize,String>{
        if world.global.world_coordinates{return Err("Sidewalks require a standard OMSI map".into());}
        if pieces.is_empty()||pieces.len()>4000{return Err("Invalid sidewalk preview".into());}
        let original=if let Some(key)=existing {
            let s=world.spline_edits.lock().current(key).ok_or("Sidewalk no longer exists")?;
            if s.deleted{return Err("Sidewalk has been deleted".into());}
            if !detach&&(s.prev_id!=0||s.next_id!=0){return Err("Sidewalk is connected. Enable Disconnect old sidewalk links to realign it".into());}
            if pieces[0].0!=key.0{return Err("Existing sidewalk start would change tiles; create as a new sidewalk".into());}
            let (tile,_)=world.editor_row_source(key.0)?;let ordinal=tile.splines.iter().position(|p|p.id==key.1);
            if ordinal.is_some_and(|i|tile.spline_attachments.iter().any(|a|a.spline_index==i as i32)) {return Err("Sidewalk has attached objects; create as new sidewalk to keep their references".into());}
            Some((key,s))
        }else{None};
        let mut neighbors:Vec<(Key,MapSpline,MapSpline)>=Vec::new();
        if let Some((key,old))=&original {if detach {
            let links:HashSet<_>=[old.prev_id,old.next_id].into_iter().filter(|id|*id!=0&&*id!=old.id).collect();
            for id in links {let mut found=Vec::new();
                for (_,tx,ty,_) in world.map_tiles(){let(mut tile,_)=world.editor_row_source((tx,ty))?;world.spline_edits.lock().overlay((tx,ty),&mut tile);
                    for s in tile.splines.into_iter().filter(|s|s.id==id&&!s.deleted){found.push((((tx,ty),id),s));}}
                if found.len()!=1{return Err(format!("Old sidewalk connection {id} missing or ambiguous; unchanged"));}
                let(k,before)=found.pop().unwrap();let mut after=before.clone();
                if after.prev_id==key.1{after.prev_id=0;}if after.next_id==key.1{after.next_id=0;}neighbors.push((k,before,after));
            }
        }}
        let mut checked=HashSet::new();
        for (tile,s) in &pieces {
            if !valid_spline(s){return Err("Invalid sidewalk geometry".into());}
            let c=curve(*tile,s);for i in 0..=8 {let t=tile_at(c.point_at(s.length*i as f64/8.0));
                if checked.insert(t){let path=world.tile_source(t.0,t.1).ok_or("Sidewalk leaves the existing map")?;let data=Tile::load(&path).map_err(|e|e.to_string())?;
                    if data.version!=0&&data.version<14{return Err("Sidewalk requires tile version 14".into());}}
            }
        }
        for (i,(_,s)) in pieces.iter_mut().enumerate(){s.id=if i==0 {original.as_ref().map(|(k,_)|k.1).or_else(||world.allocate_editor_id())}else{world.allocate_editor_id()}.ok_or("No free spline ID")?;
            if let Some((_,old))=&original{s.rules=old.rules.clone();}}
        let mut previous=0;
        for i in 0..pieces.len(){let end=pieces[i].1.next_id==-1;pieces[i].1.prev_id=previous;pieces[i].1.next_id=if end||i+1==pieces.len(){0}else{pieces[i+1].1.id};previous=if end{0}else{pieces[i].1.id};}
        let mut before:Vec<_>=pieces.iter().enumerate().map(|(i,(tile,s))|{if i==0 {if let Some(old)=&original{return old.clone();}}let mut absent=s.clone();absent.deleted=true;((*tile,s.id),absent)}).collect();
        before.extend(neighbors.iter().map(|(k,b,_)|(*k,b.clone())));
        self.remember(before);let mut edits=world.spline_edits.lock();let count=pieces.len();let first=(pieces[0].0,pieces[0].1.id);
        for (i,(tile,s)) in pieces.into_iter().enumerate(){let key=(tile,s.id);if i==0&&original.is_some()&&!edits.added.contains_key(&key){edits.changed.insert(key,s);}else{edits.originals.entry(key).or_insert_with(||s.clone());edits.added.insert(key,s);}edits.dirty_tiles.insert(tile);}
        for(key,before,after)in neighbors{if edits.added.contains_key(&key){edits.added.insert(key,after);}else{edits.originals.entry(key).or_insert(before);edits.changed.insert(key,after);}edits.dirty_tiles.insert(key.0);}
        edits.dirty=true;self.selected=Some(first);Ok(count)
    }

    fn add(&mut self, world: &World, file: &str, start: DVec3, end: DVec3, offset: f64) -> String {
        let Some(ty) = world.spline_type(file) else { return format!("Spline type not found: {file}") };
        if (ty.def.only_editor || ty.def.profiles.is_empty()) && ty.def.paths.is_empty() { return "Choose a road or traffic path type".into(); }
        if omsi_map::world_coordinates() { return "Spline editing on world-coordinate maps is not supported yet".into(); }
        let length = (end - start).truncate().length();
        if !length.is_finite() || !(0.5..=500.0).contains(&length) { return "Distance must be 0.5 to 500 m".into(); }
        let cuts = straight_tile_cuts(start, end);
        let mut sections = Vec::new();
        for pair in cuts.windows(2) {
            let a = start.lerp(end, pair[0]);
            let b = start.lerp(end, pair[1]);
            let tile = tile_at((a + b) * 0.5);
            let Some(source) = world.tile_source(tile.0, tile.1) else { return "The road leaves the map; nothing created".into() };
            let Ok(base) = Tile::load(&source) else { return "Cannot read tile".into() };
            if base.version != 0 && base.version < 14 { return "Spline editing needs a version 14 tile".into(); }
            let Some(id) = world.allocate_editor_id() else { return "No free map ID".into() };
            let mut piece=between(file,id,tile,a,b,offset+length*pair[0]);
            if ty.def.only_editor || ty.def.profiles.is_empty() {
                for (i,path) in ty.def.paths.iter().enumerate().filter(|(_,p)|p.kind==0) {
                    let _=path;
                    for kind in ["bus","trucks"] {piece.rules.push(omsi_map::MapRule {path_index:i as i32,kind:kind.into(),value:1.0,..Default::default()});}
                }
            }
            sections.push((tile,piece));
        }
        for i in 0..sections.len() {
            sections[i].1.prev_id = if i == 0 { 0 } else { sections[i - 1].1.id };
            sections[i].1.next_id = if i + 1 == sections.len() { 0 } else { sections[i + 1].1.id };
        }
        let before = sections.iter().map(|(tile, s)| {
            let mut deleted = s.clone(); deleted.deleted = true;
            ((*tile, s.id), deleted)
        }).collect();
        self.remember(before);
        let mut edits = world.spline_edits.lock();
        let count = sections.len();
        for (tile, s) in sections {
            let key = (tile, s.id);
            edits.originals.insert(key, s.clone());
            edits.added.insert(key, s);
            edits.dirty_tiles.insert(tile);
            self.selected = Some(key);
        }
        edits.dirty = true;
        drop(edits);
        format!("New spline: {count} section(s) · Ctrl+S to save · Then reload map")
    }

    pub fn begin_drag(&mut self, world: &World, ground: DVec3) {
        self.drag_offset = None;
        self.drag_before = None;
        if self.connection_active() { return; }
        if let Some((key, s)) = self.selected.and_then(|k| world.spline_edits.lock().current(k).map(|s| (k, s))) {
            if s.deleted { return; }
            self.drag_offset = Some(curve(key.0, &s).point_at(0.0) - ground);
            self.drag_before = Some((key, s));
        }
    }

    pub fn finish_drag(&mut self) { self.drag_offset = None; self.drag_before = None; }

    pub fn drag_to(&mut self, world: &World, ground: DVec3) -> Option<String> {
        let key = self.selected?;
        let offset = self.drag_offset?;
        let mut s = world.spline_edits.lock().current(key)?;
        let before_move = s.clone();
        let at = ground + offset;
        let origin = origin(key.0);
        s.pos = [at.x - origin.x, at.y - origin.y, at.z];
        if s != before_move && inside_tile(&s) {
            if let Some(before) = self.drag_before.take() { self.remember(vec![before]); }
        }
        self.set(world, key, s)
    }
}

fn origin(tile: (i32, i32)) -> DVec2 {
    DVec2::new(tile.0 as f64, tile.1 as f64) * omsi_map::tile_size()
}

fn curve(tile: (i32, i32), s: &MapSpline) -> SplineCurve {
    SplineCurve::from_map(s, origin(tile))
}

const ROAD_CLEARANCE: f64 = 0.05;

fn fitted_height(terrain: f64, center: DVec3, profile: &Roadbed) -> f64 {
    // A .sli can put its asphalt above its reference height. Subtract that
    // offset so repeated fitting does not lift the road by its own profile height.
    let offset = profile.nearest(center.truncate(), 1000.0).map(|(_, z)| z - center.z).unwrap_or(0.0);
    terrain + ROAD_CLEARANCE - offset
}

#[derive(Default)]
struct Roadbed { faces: Vec<RoadFace> }

struct RoadFace { points: [DVec3; 3], bounds: [f64; 4] }

fn bounds_distance_squared(p: DVec2, b: [f64; 4]) -> f64 {
    let dx = (b[0] - p.x).max(p.x - b[2]).max(0.0);
    let dy = (b[1] - p.y).max(p.y - b[3]).max(0.0);
    dx * dx + dy * dy
}

impl RoadFace {
    fn nearest(&self, p: DVec2) -> (f64, f64) {
        let [a, b, c] = self.points;
        let (ab, ac, ap) = (b.truncate() - a.truncate(), c.truncate() - a.truncate(), p - a.truncate());
        let determinant = ab.perp_dot(ac);
        let (u, v) = (ap.perp_dot(ac) / determinant, ab.perp_dot(ap) / determinant);
        if u >= -1e-9 && v >= -1e-9 && u + v <= 1.0 + 1e-9 {
            return (0.0, a.z + u * (b.z - a.z) + v * (c.z - a.z));
        }
        [(a, b), (b, c), (c, a)].into_iter().map(|(a, b)| {
            let edge = b.truncate() - a.truncate();
            let t = ((p - a.truncate()).dot(edge) / edge.length_squared().max(1e-12)).clamp(0.0, 1.0);
            let q = a.lerp(b, t);
            ((q.truncate() - p).length_squared(), q.z)
        }).min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1))).unwrap()
    }
}

impl Roadbed {
    fn add_mesh(&mut self, mesh: &omsi_geometry::MeshData, o: DVec3) {
        for indices in mesh.indices.chunks_exact(3) {
            let p = [0, 1, 2].map(|i| o + mesh.positions[indices[i] as usize].as_dvec3());
            if p.iter().any(|p| !p.is_finite()) { continue; }
            if (p[1] - p[0]).truncate().perp_dot((p[2] - p[0]).truncate()).abs() < 1e-8 { continue; }
            let bounds = p.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p|
                [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]);
            self.faces.push(RoadFace { points: p, bounds });
        }
    }

    fn bounds(&self) -> [f64; 4] {
        self.faces.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, f|
            [b[0].min(f.bounds[0]), b[1].min(f.bounds[1]), b[2].max(f.bounds[2]), b[3].max(f.bounds[3])])
    }

    fn touches_tile(&self, key: (i32, i32), reach: f64) -> bool {
        let o = origin(key); let size = omsi_map::tile_size();
        self.faces.iter().any(|f| f.bounds[0] <= o.x + size + reach && f.bounds[2] >= o.x - reach
            && f.bounds[1] <= o.y + size + reach && f.bounds[3] >= o.y - reach)
    }

    fn nearest(&self, p: DVec2, reach: f64) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for face in &self.faces {
            if bounds_distance_squared(p, face.bounds) > best.map_or(reach * reach, |b| b.0) + 1e-8 { continue; }
            let hit = face.nearest(p);
            if hit.0 > reach * reach { continue; }
            if best.is_none_or(|b| hit.0 < b.0 - 1e-8 || ((hit.0 - b.0).abs() <= 1e-8 && hit.1 < b.1)) { best = Some(hit); }
        }
        best
    }
}

fn terrain_plan(bed: &Roadbed, before: &HashMap<(i32, i32), omsi_map::Terrain>, guard: f64, shoulder: f64) -> TerrainPlan {
    let mut after = before.clone();
    for (key, t) in &mut after {
        let o = origin(*key); let n = t.samples(); let cell = omsi_map::tile_size() / t.cells as f64;
        for iy in 0..n {
            for ix in 0..n {
                let p = o + DVec2::new(ix as f64 * cell, iy as f64 * cell);
                let Some((d2, road_height)) = bed.nearest(p, guard + shoulder) else { continue };
                let fade = ((d2.sqrt() - guard) / shoulder).clamp(0.0, 1.0);
                let weight = 1.0 - fade * fade * (3.0 - 2.0 * fade);
                let h = &mut t.heights[iy * n + ix];
                *h = (*h as f64 + (road_height - ROAD_CLEARANCE - *h as f64) * weight) as f32;
            }
        }
        lower_under_road(t, *key, bed);
    }
    match_terrain_edges(&mut after, before);
    let mut changed: Vec<_> = after.into_iter().filter(|(key, t)| before.get(key) != Some(t)).collect();
    changed.sort_by_key(|(key, _)| *key);
    let old = changed.iter().map(|(key, _)| (*key, before[key].clone())).collect();
    TerrainPlan { before: old, after: changed }
}

/// Clip a road triangle against the terrain's actual triangle. Its height travels
/// with every intersection point; checking these polygon vertices bounds the whole
/// overlap, rather than just checking the centre or a few raster samples.
fn clip_road_face(points: [DVec3; 3], triangle: [DVec2; 3]) -> Vec<DVec3> {
    let mut polygon = points.to_vec();
    for i in 0..3 {
        let (a, b) = (triangle[i], triangle[(i + 1) % 3]);
        let distance = |p: DVec3| (b - a).perp_dot(p.truncate() - a);
        let Some(mut previous) = polygon.last().copied() else { break };
        let mut d0 = distance(previous); let mut clipped = Vec::new();
        for current in polygon {
            let d1 = distance(current);
            if (d0 >= 0.0) != (d1 >= 0.0) { clipped.push(previous.lerp(current, (d0 / (d0 - d1)).clamp(0.0, 1.0))); }
            if d1 >= 0.0 { clipped.push(current); }
            previous = current; d0 = d1;
        }
        polygon = clipped;
    }
    polygon
}

fn lower_under_road(t: &mut omsi_map::Terrain, key: (i32, i32), bed: &Roadbed) {
    let o = origin(key); let n = t.samples(); let cell = omsi_map::tile_size() / t.cells as f64;
    let max_cell = t.cells.saturating_sub(1) as f64;
    for face in &bed.faces {
        let b = face.bounds; let size = omsi_map::tile_size();
        if b[2] < o.x || b[0] > o.x + size || b[3] < o.y || b[1] > o.y + size { continue; }
        let (ix0, ix1) = (((b[0] - o.x) / cell).floor().clamp(0.0, max_cell) as usize, ((b[2] - o.x) / cell).floor().clamp(0.0, max_cell) as usize);
        let (iy0, iy1) = (((b[1] - o.y) / cell).floor().clamp(0.0, max_cell) as usize, ((b[3] - o.y) / cell).floor().clamp(0.0, max_cell) as usize);
        for iy in iy0..=iy1 {
            for ix in ix0..=ix1 {
                for vertices in [[(ix, iy), (ix + 1, iy), (ix + 1, iy + 1)], [(ix, iy), (ix + 1, iy + 1), (ix, iy + 1)]] {
                    let triangle = vertices.map(|(x, y)| o + DVec2::new(x as f64 * cell, y as f64 * cell));
                    let ids = vertices.map(|(x, y)| y * n + x);
                    let polygon = clip_road_face(face.points, triangle);
                    let ab = triangle[1] - triangle[0]; let ac = triangle[2] - triangle[0]; let area = ab.perp_dot(ac);
                    let heights = ids.map(|id| t.heights[id] as f64);
                    let excess = polygon.iter().fold(0.0f64, |excess, p| {
                        let q = p.truncate() - triangle[0]; let u = q.perp_dot(ac) / area; let v = ab.perp_dot(q) / area;
                        let h = heights[0] + u * (heights[1] - heights[0]) + v * (heights[2] - heights[0]);
                        excess.max(h - p.z + ROAD_CLEARANCE)
                    });
                    if excess > 1e-5 { for id in ids { t.heights[id] = (t.heights[id] as f64 - excess - 1e-4) as f32; } }
                }
            }
        }
    }
}

/// Only reconcile shared border nodes touched by this operation. Taking the lower
/// result keeps every road-clearance constraint and avoids a crack between tiles.
fn match_terrain_edges(after: &mut HashMap<(i32, i32), omsi_map::Terrain>, before: &HashMap<(i32, i32), omsi_map::Terrain>) {
    let mut touched = HashSet::new();
    let coordinate = |key: (i32, i32), ix: usize, iy: usize, cells: usize| {
        let p = origin(key) + DVec2::new(ix as f64, iy as f64) * (omsi_map::tile_size() / cells as f64);
        ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64)
    };
    for (key, t) in after.iter() {
        let n = t.samples();
        for iy in 0..n { for ix in 0..n {
            if ix != 0 && iy != 0 && ix != t.cells && iy != t.cells { continue; }
            if t.heights[iy * n + ix] != before[key].heights[iy * n + ix] { touched.insert(coordinate(*key, ix, iy, t.cells)); }
        } }
    }
    let mut heights: HashMap<(i64, i64), f32> = HashMap::new();
    for (key, t) in after.iter() {
        let n = t.samples();
        for iy in 0..n { for ix in 0..n {
            if ix != 0 && iy != 0 && ix != t.cells && iy != t.cells { continue; }
            let p = coordinate(*key, ix, iy, t.cells);
            if touched.contains(&p) { heights.entry(p).and_modify(|h| *h = (*h).min(t.heights[iy * n + ix])).or_insert(t.heights[iy * n + ix]); }
        } }
    }
    for (key, t) in after.iter_mut() {
        let n = t.samples();
        for iy in 0..n { for ix in 0..n {
            if ix != 0 && iy != 0 && ix != t.cells && iy != t.cells { continue; }
            if let Some(h) = heights.get(&coordinate(*key, ix, iy, t.cells)) { t.heights[iy * n + ix] = *h; }
        } }
    }
}

fn tile_at(p: DVec3) -> (i32, i32) {
    let size = omsi_map::tile_size();
    ((p.x / size).floor() as i32, (p.y / size).floor() as i32)
}

/// Exact grid crossings of a straight line, including negative tile coordinates.
fn straight_tile_cuts(start: DVec3, end: DVec3) -> Vec<f64> {
    let size = omsi_map::tile_size();
    let mut cuts = vec![0.0, 1.0];
    for (a, b) in [(start.x, end.x), (start.y, end.y)] {
        if (b - a).abs() < 1e-9 { continue; }
        let lo = (a.min(b) / size).floor() as i32;
        let hi = (a.max(b) / size).floor() as i32;
        for n in lo + 1..=hi {
            let t = (n as f64 * size - a) / (b - a);
            if t > 1e-9 && t < 1.0 - 1e-9 { cuts.push(t); }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    cuts
}

fn nearest_station(c: &SplineCurve, at: DVec3) -> f64 {
    let count = (c.length / 2.0).ceil().clamp(1.0, 4096.0) as usize;
    let step = c.length / count as f64;
    let distance = |d| (c.point_at(d).truncate() - at.truncate()).length_squared();
    let mut best = 0.0;
    for i in 1..=count {
        let d = i as f64 * step;
        if distance(d) < distance(best) { best = d; }
    }
    let (mut lo, mut hi) = ((best - step).max(0.0), (best + step).min(c.length));
    for _ in 0..32 {
        let a = lo + (hi - lo) / 3.0;
        let b = hi - (hi - lo) / 3.0;
        if distance(a) < distance(b) { hi = b; } else { lo = a; }
    }
    let middle = (lo + hi) * 0.5;
    [0.0, middle, c.length].into_iter().min_by(|a, b| distance(*a).total_cmp(&distance(*b))).unwrap_or(0.0)
}

// The renderer and handles must share one geometry path (including skew and mirror).
fn physical_edges(def: &omsi_scenery::Spline, mirror: bool) -> [(f64, f64); 2] {
    let (left, right) = profile_edges(def);
    if mirror { [(-right.0, right.1), (-left.0, left.1)] } else { [left, right] }
}

fn edge_points(tile: (i32, i32), s: &MapSpline, def: &omsi_scenery::Spline, end: End) -> [DVec3; 2] {
    let station = if end == End::Start { 0.0 } else { s.length };
    let c = curve(tile, s);
    physical_edges(def, s.mirror).map(|(x, z)|
        omsi_geometry::spline_profile_point(def, &c, false, station, x, z))
}

fn edge_markers(world: &World, tile: (i32, i32), s: &MapSpline, steps: usize) -> Vec<DVec3> {
    let Some(ty) = world.spline_type(&s.file) else { return Vec::new() };
    let c = curve(tile, s);
    let edges = physical_edges(&ty.def, s.mirror);
    (0..=steps).flat_map(|i| edges.map(|(x, z)| omsi_geometry::spline_profile_point(
        &ty.def, &c, false, s.length * i as f64 / steps as f64, x, z))).collect()
}

/// Keep profiles and materials intact; a width taper may only join compatible
/// cross-sections. Never pretend that a sidewalk, lane merge or curb was invented.
fn compatible_sections(a: &omsi_scenery::Spline, am: bool, b: &omsi_scenery::Spline, bm: bool, reverse: bool) -> bool {
    let ae = physical_edges(a, am);
    let be = physical_edges(b, bm);
    let aw = ae[1].0 - ae[0].0;
    let bw = be[1].0 - be[0].0;
    if aw <= 0.001 || bw <= 0.001 { return false; }
    // Compare every surface vertex with the target surface at its normalised width.
    // The two directions detect extra target curbs/borders too.
    let check = |from: &omsi_scenery::Spline, fm: bool, to: &omsi_scenery::Spline, tm: bool,
                 fe: [(f64, f64); 2], te: [(f64, f64); 2]| {
        from.profiles.iter().filter(|p| p.points.len() >= 2).flat_map(|p| &p.points).all(|p| {
            let x = p.x as f64 * if fm { -1.0 } else { 1.0 };
            let t = (x - fe[0].0) / (fe[1].0 - fe[0].0);
            let u = if reverse { 1.0 - t } else { t };
            let tx = te[0].0 + u * (te[1].0 - te[0].0);
            let residual = p.z as f64 - (fe[0].1 * (1.0 - t) + fe[1].1 * t);
            to.profiles.iter().flat_map(|p| p.points.windows(2)).any(|pair| {
                let x0 = pair[0].x as f64 * if tm { -1.0 } else { 1.0 };
                let x1 = pair[1].x as f64 * if tm { -1.0 } else { 1.0 };
                if tx < x0.min(x1) - 1e-4 || tx > x0.max(x1) + 1e-4 { return false; }
                let target_base = te[0].1 * (1.0 - u) + te[1].1 * u;
                if (x1 - x0).abs() < 1e-6 {
                    let z = residual + target_base;
                    return z >= (pair[0].z.min(pair[1].z) as f64) - 0.005
                        && z <= (pair[0].z.max(pair[1].z) as f64) + 0.005;
                }
                let f = ((tx - x0) / (x1 - x0)).clamp(0.0, 1.0);
                let z = pair[0].z as f64 * (1.0 - f) + pair[1].z as f64 * f;
                (z - target_base - residual).abs() <= 0.005
            })
        })
    };
    check(a, am, b, bm, ae, be) && check(b, bm, a, am, be, ae)
}

fn align_connection_edges(source_tile: (i32, i32), original: &MapSpline, source_def: &omsi_scenery::Spline,
    target_tile: (i32, i32), target_def: &omsi_scenery::Spline, preview: &mut ConnectionGeometry,
    allow_transition: bool) -> Result<(), String> {
    align_connection_surface(source_tile,original,source_def,target_tile,target_def,preview,allow_transition,None)
}
fn align_connection_surface(source_tile:(i32,i32),original:&MapSpline,source_def:&omsi_scenery::Spline,
    target_tile:(i32,i32),target_def:&omsi_scenery::Spline,preview:&mut ConnectionGeometry,
    allow_transition:bool,surface:Option<&dyn Fn(f64,f64)->DVec3>)->Result<(),String> {
    let end = if preview.source_end == End::Start { 0 } else { 1 };
    let source_edges = physical_edges(source_def, preview.source.mirror);
    let target_edges = physical_edges(target_def, preview.target.mirror);
    let width = source_edges[1].0 - source_edges[0].0;
    let target_width = target_edges[1].0 - target_edges[0].0;
    if !width.is_finite() || !target_width.is_finite() || width < 0.01 || target_width < 0.01 {
        return Err("Both profiles need two visible side edges".into());
    }
    if (width - target_width).abs() > 0.01 && !allow_transition {
        return Err(format!("Widths {width:.2}/{target_width:.2} m: optionally enable profile transition"));
    }
    let reverse = preview.source_end == preview.target_end;
    if !compatible_sections(source_def, preview.source.mirror, target_def, preview.target.mirror, reverse) {
        return Err("Profile shapes do not match (e.g. different kerb). A matching transition profile is required".into());
    }
    // Recompute from the undeformed end, so repeated connect is idempotent.
    preview.source.profile_transitions[end] = None;
    let actual = edge_points(source_tile, &preview.source, source_def, preview.source_end);
    let mut goal = if let Some(surface)=surface {target_edges.map(|(x,z)|surface(x,z))} else {edge_points(target_tile, &preview.target, target_def, preview.target_end)};
    if reverse { goal.swap(0, 1); }
    let c = curve(source_tile, &preview.source);
    let forward = SplineCurve::dir(preview.source_end.heading(&c));
    let right = DVec2::new(forward.y, -forward.x);
    let offsets = [0, 1].map(|i| {
        let d = goal[i] - actual[i];
        [d.truncate().dot(right), d.truncate().dot(forward), d.z]
    });
    let lateral = offsets.iter().map(|d| d[0].abs()).fold(0.0f64, f64::max);
    let correction = offsets.iter().flatten().map(|v| v.abs()).fold(0.0f64, f64::max);
    if correction > 50.0 || !correction.is_finite() { return Err("Profile transition would be too large".into()); }
    if correction > 1e-7 {
        let available = preview.source.length * 0.5;
        let longitudinal=offsets.iter().map(|d|d[1].abs()).fold(0.0f64,f64::max);
        let required = (lateral * 5.0).max(longitudinal * 3.0).max(0.5);
        if required > available + 1e-6 {
            return Err(format!("Use a source spline at least {:.1} m long for this transition", required * 2.0));
        }
        preview.source.profile_transitions[end] = Some(omsi_map::ProfileTransition {
            station: if end == 0 { 0.0 } else { preview.source.length },
            span: 10.0f64.min(available).max(required),
            x: [source_edges[0].0, source_edges[1].0], offsets,
        });
    }
    let fitted = edge_points(source_tile, &preview.source, source_def, preview.source_end);
    if fitted.iter().zip(goal).any(|(a, b)| a.distance(b) > 0.001) {
        return Err("Could not align edge points flush".into());
    }
    // Edges alone do not prove that the asphalt/curb vertices between them meet.
    // Check those too, including the renderer's cant clipping and skewed curves.
    let target_curve = curve(target_tile, &preview.target);
    let target_station = if preview.target_end == End::Start { 0.0 } else { preview.target.length };
    let source_curve = curve(source_tile, &preview.source);
    let source_station = if preview.source_end == End::Start { 0.0 } else { preview.source.length };
    for p in source_def.profiles.iter().filter(|p| p.points.len() >= 2).flat_map(|p| &p.points) {
        let sx = p.x as f64 * if preview.source.mirror { -1.0 } else { 1.0 };
        let t = (sx - source_edges[0].0) / width;
        let u = if reverse { 1.0 - t } else { t };
        let tx = target_edges[0].0 + u * target_width;
        let source_point = omsi_geometry::spline_profile_point(source_def, &source_curve, false, source_station, sx, p.z as f64);
        let matches = target_def.profiles.iter().flat_map(|p| p.points.windows(2)).any(|pair| {
            let x = pair.iter().map(|p| p.x as f64 * if preview.target.mirror { -1.0 } else { 1.0 }).collect::<Vec<_>>();
            if tx < x[0].min(x[1]) - 1e-4 || tx > x[0].max(x[1]) + 1e-4 { return false; }
            let points = [0, 1].map(|i| if let Some(surface)=surface {surface(x[i],pair[i].z as f64)} else {omsi_geometry::spline_profile_point(target_def, &target_curve, false,
                target_station, x[i], pair[i].z as f64)});
            let v = points[1] - points[0];
            let f = ((source_point - points[0]).dot(v) / v.length_squared().max(1e-12)).clamp(0.0, 1.0);
            source_point.distance(points[0] + v * f) <= 0.02
        });
        if !matches { return Err("Inner profile or crossfall does not match; a matching transition profile is required".into()); }
    }
    // Existing connections at the far end must not be torn apart by a new fit.
    let far = if end == 0 { End::Finish } else { End::Start };
    if far.link(original) != 0 {
        let old = edge_points(source_tile, original, source_def, far);
        let new = edge_points(source_tile, &preview.source, source_def, far);
        if old.iter().zip(new).any(|(a, b)| a.distance(b) > 0.005) {
            return Err("The other connection would detach; use a free section first".into());
        }
    }
    Ok(())
}

fn profile_edges(s: &omsi_scenery::Spline) -> ((f64, f64), (f64, f64)) {
    let mut left = (f64::INFINITY, 0.0);
    let mut right = (f64::NEG_INFINITY, 0.0);
    for p in s.profiles.iter().flat_map(|p| &p.points) {
        if (p.x as f64) < left.0 { left = (p.x as f64, p.z as f64); }
        if (p.x as f64) > right.0 { right = (p.x as f64, p.z as f64); }
    }
    if !left.0.is_finite() {
        for p in &s.paths {
            let lo=p.start[0] as f64-p.width as f64*0.5;let hi=p.start[0] as f64+p.width as f64*0.5;
            if lo<left.0{left=(lo,p.start[2] as f64);}if hi>right.0{right=(hi,p.start[2] as f64);}
        }
        if !left.0.is_finite(){return ((0.0,0.0),(0.0,0.0));}
    }
    (left, right)
}

fn arc_to(s: &MapSpline, tile: (i32, i32), target: DVec3) -> Result<MapSpline, String> {
    let start = curve(tile, s).point_at(0.0);
    let dir = SplineCurve::dir(s.heading);
    let d = (target - start).truncate();
    let forward = d.dot(dir);
    let side = d.dot(DVec2::new(dir.y, -dir.x));
    if forward < 0.5 { return Err("Target lies behind the start; align using N/M first".into()); }
    let (radius, length) = if side.abs() < 1e-6 { (0.0, forward) } else {
        let r = d.length_squared() / (2.0 * side);
        if r.abs() < 20.0 { return Err("Curve radius would be below 20 m; move or rotate start".into()); }
        let angle = (forward / r).atan2(1.0 - side / r);
        (r, r * angle)
    };
    if !(0.5..=500.0).contains(&length) { return Err("Connection must be 0.5 to 500 m long".into()); }
    let mut out = s.clone();
    out.radius = radius;
    out.length = length;
    out.delta_h = Some(target.z - start.z);
    out.is_h = true;
    Ok(out)
}

fn closest_ends(source_tile: (i32, i32), source: &MapSpline, target_tile: (i32, i32), target: &MapSpline) -> (End, End, f64) {
    let a = curve(source_tile, source);
    let b = curve(target_tile, target);
    [(End::Start, End::Start), (End::Start, End::Finish), (End::Finish, End::Start), (End::Finish, End::Finish)].into_iter()
        .map(|(x, y)| (x, y, (x.point(&a) - y.point(&b)).length()))
        .min_by(|x, y| x.2.total_cmp(&y.2)).unwrap()
}

/// Preview a replacement without changing any record. All reciprocal edits are
/// carried with the pair so confirmation and undo commit them as one operation.
fn connection_plan(edits: &Edits, loaded: &[(i32, i32)], index: Option<&crate::tiles::MapIndex>, source_key: Key, target_key: Key, replace: bool, allow_transition: bool) -> Result<ConnectionGeometry, String> {
    let (Some(source), Some(target)) = (edits.current(source_key), edits.current(target_key)) else {
        return Err("Spline no longer available".into());
    };
    let mut preview = if !replace && !allow_transition {connection_geometry(source_key.0,&source,target_key.0,&target)?}
        else {connection_geometry_impl(source_key.0, &source, target_key.0, &target, replace, allow_transition)?};
    let mut neighbors: HashMap<Key, MapSpline> = HashMap::new();
    for (owner, end, other_id) in [(&source, preview.source_end, target.id), (&target, preview.target_end, source.id)] {
        let old_id = end.link(owner);
        if old_id == 0 || old_id == other_id { continue; }
        let keys: HashSet<_> = edits.originals.keys().chain(edits.added.keys()).filter(|key| key.1 == old_id).copied().collect();
        if keys.len() > 1 { return Err(format!("Old connection {old_id} is ambiguous; unchanged")); }
        let Some(key) = keys.into_iter().next() else {
            let Some(index) = index else { return Err("Check old connection first".into()) };
            if index.splines.contains_key(&old_id) {
                return Err(format!("Load old neighbouring spline {old_id} for editing first"));
            }
            if index.tiles_failed != 0 {
                return Err("Map not fully read; leave old connection unchanged".into());
            }
            // Missing is established by the whole-map index, never by the cache
            // of loaded tiles alone. The user still has to confirm replacement.
            continue;
        };
        if key == source_key || key == target_key { continue; }
        if !loaded.contains(&key.0) { return Err(format!("Load old neighbouring spline {old_id} first")); }
        let mut neighbor = neighbors.get(&key).cloned().or_else(|| edits.current(key)).ok_or_else(|| format!("Old neighbouring spline {old_id} unavailable"))?;
        let reciprocal: Vec<_> = [End::Start, End::Finish].into_iter().filter(|end| end.link(&neighbor) == owner.id).collect();
        if reciprocal.len() > 1 { return Err(format!("Neighbour {old_id} has two return links; connection ambiguous")); }
        if let Some(end) = reciprocal.first() {
            end.set_link(&mut neighbor, 0);
            neighbors.insert(key, neighbor);
        }
    }
    preview.neighbors = neighbors.into_iter().collect();
    preview.neighbors.sort_by_key(|(key, _)| *key);
    Ok(preview)
}

/// Join the nearest two endpoints. Preserve the source's opposite endpoint and its
/// tangent; the target's geometry and both record IDs/attachment indices stay intact.
fn connection_geometry(source_tile: (i32, i32), source: &MapSpline, target_tile: (i32, i32), target: &MapSpline) -> Result<ConnectionGeometry, String> {
    connection_geometry_impl(source_tile, source, target_tile, target, false, false)
}

fn connection_geometry_impl(source_tile: (i32, i32), source: &MapSpline, target_tile: (i32, i32), target: &MapSpline, replace: bool, allow_transition: bool) -> Result<ConnectionGeometry, String> {
    if source.deleted || target.deleted || !valid_spline(source) || !valid_spline(target) {
        return Err("Both splines must exist and be valid".into());
    }
    if source.id == target.id { return Err("Select two different splines".into()); }
    let b = curve(target_tile, target);
    let (source_end, target_end, gap) = closest_ends(source_tile, source, target_tile, target);
    if gap > 50.0 { return Err("Spline ends are more than 50 m apart".into()); }
    if !replace {
        for (spline, end, other_id) in [(source, source_end, target.id), (target, target_end, source.id)] {
            if ![0, other_id].contains(&end.link(spline)) {
                return Err(format!("Spline {} {} occupied by {}", spline.id,
                    if end == End::Start { "Start" } else { "End" }, end.link(spline)));
            }
        }
    }
    let point = target_end.point(&b);
    let same_end = source_end == target_end;
    let direction = if same_end { -1.0 } else { 1.0 };
    let target_distance = if target_end == End::Start { 0.0 } else { target.length };
    let grade = b.slope_at(target_distance) * 100.0 * direction;
    let cant = b.cant_at(target_distance) * direction;
    let mut fitted = fit_connection_endpoint(source_tile, source, source_end, point, grade, cant)?;
    let desired_heading = target_end.heading(&b) + if same_end { 180.0 } else { 0.0 };
    let difference = (source_end.heading(&curve(source_tile, &fitted)) - desired_heading + 180.0).rem_euclid(360.0) - 180.0;
    if difference.abs() > if allow_transition {30.0} else {2.0} {
        return Err(if allow_transition {"End directions differ by more than 30°; insert an additional curve section"}
            else {"End directions do not match; enable profile / curve transition"}.into());
    }
    if !valid_spline(&fitted) { return Err("Connection has invalid values".into()); }
    let mut linked_target = target.clone();
    source_end.set_link(&mut fitted, target.id);
    target_end.set_link(&mut linked_target, source.id);
    Ok(ConnectionGeometry { source: fitted, target: linked_target, source_end, target_end, gap, neighbors: Vec::new() })
}

fn fit_connection_endpoint(source_tile: (i32, i32), source: &MapSpline, source_end: End, point: DVec3, grade: f64, cant: f64) -> Result<MapSpline, String> {
    let a = curve(source_tile, source);
    let fitted = if source_end == End::Finish {
        let mut fitted = arc_to(source, source_tile, point)?;
        fitted.grad_end = grade;
        fitted.cant_end = cant;
        fitted
    } else {
        // Solve from the fixed far endpoint in reverse, then put the result back in
        // its original authored direction. Do not reverse profiles, rules or lanes.
        let anchor = a.end_point();
        let o = origin(source_tile);
        let mut reverse = source.clone();
        reverse.pos = [anchor.x - o.x, anchor.y - o.y, anchor.z];
        reverse.heading = (a.heading_at(source.length) + 180.0).rem_euclid(360.0);
        reverse.radius = -source.radius;
        reverse.grad_start = -source.grad_end;
        reverse.grad_end = -source.grad_start;
        reverse.delta_h = Some(a.start.z - anchor.z);
        reverse.is_h = true;
        let reversed = arc_to(&reverse, source_tile, point)?;
        let c = curve(source_tile, &reversed);
        let mut fitted = source.clone();
        fitted.pos = [point.x - o.x, point.y - o.y, point.z];
        fitted.heading = (c.heading_at(reversed.length) + 180.0).rem_euclid(360.0);
        fitted.radius = -reversed.radius;
        fitted.length = reversed.length;
        fitted.grad_start = grade;
        fitted.cant_start = cant;
        fitted.delta_h = Some(anchor.z - point.z);
        fitted.is_h = true;
        fitted.tex_offset = source.tex_offset + source.length - fitted.length;
        fitted.map_chain_offset = Some(fitted.tex_offset);
        fitted
    };
    Ok(fitted)
}

/// The target is the arm's start looking inward; it is not inserted as a spline.
fn deleted_junction_neighbor(link:i64,junction:i64,matches:&[MapSpline])->bool{
    link!=0&&link!=junction&&matches.len()==1&&matches[0].id==link&&matches[0].deleted
}
fn junction_end(source:&MapSpline,c:&SplineCurve,id:i64,point:DVec3)->End {
    if source.prev_id==id {End::Start} else if source.next_id==id {End::Finish}
    else if c.start.distance(point)<c.end_point().distance(point){End::Start}else{End::Finish}
}
#[cfg(test)]
fn junction_fit(tile:(i32,i32),source:&MapSpline,source_def:&omsi_scenery::Spline,id:i64,point:DVec3,outward:f64,target_def:&omsi_scenery::Spline)->Result<MapSpline,String>{
    let port=crate::junction_connections::Pose{at:point,heading:0.0,tilt:[0.0;2]}.port(DVec3::ZERO,outward)?;
    junction_fit_port(tile,source,source_def,id,port,target_def)
}
fn junction_fit_port(tile:(i32,i32),source:&MapSpline,source_def:&omsi_scenery::Spline,id:i64,port:crate::junction_connections::Port,target_def:&omsi_scenery::Spline)->Result<MapSpline,String>{
    if source.deleted||!valid_spline(source){return Err("Invalid road".into());}
    if source.prev_id==id&&source.next_id==id{return Err("Road is connected to this junction at both ends; edit separately".into());}
    let c=curve(tile,source);let end=junction_end(source,&c,id,port.point);
    let gap=end.point(&c).distance(port.point);if gap>50.0{return Err(format!("Road {}: {:.2} m distance to arm (max. 50 m). Select a matching road",source.id,gap));}
    if ![0,id].contains(&end.link(source)){return Err(format!("Road {}: {} occupied by ID {}. Active connection not overwritten",source.id,if end==End::Start{"Start"}else{"End"},end.link(source)));}
    let sign=if end==End::Finish{-1.0}else{1.0};
    let mut fitted=fit_connection_endpoint(tile,source,end,port.point,port.grade*sign,port.cant*sign)?;
    let desired=port.outward+if end==End::Finish{180.0}else{0.0};
    let delta=(end.heading(&curve(tile,&fitted))-desired+180.0).rem_euclid(360.0)-180.0;
    if delta.abs()>30.0{return Err("Connection angle above 30°; align road or junction arm first".into());}
    end.set_link(&mut fitted,id);
    let o=origin(tile);let target=MapSpline{id,pos:[port.point.x-o.x,port.point.y-o.y,port.point.z],heading:(port.outward+180.0).rem_euclid(360.0),length:1.0,grad_start:-port.grade,grad_end:-port.grade,cant_start:-port.cant,cant_end:-port.cant,..Default::default()};
    let mut preview=ConnectionGeometry{source:fitted,target,source_end:end,target_end:End::Start,gap,neighbors:Vec::new()};
    align_connection_surface(tile,source,source_def,tile,target_def,&mut preview,true,Some(&|x,z|port.surface(x,z)))?;
    if !valid_spline(&preview.source){return Err("Invalid junction connection".into());}Ok(preview.source)
}

fn connection_tiles(source: Key, target: Key, fitted: &MapSpline) -> HashSet<(i32, i32)> {
    let mut tiles = HashSet::from([source.0, target.0]);
    let c = curve(source.0, fitted);
    let count = (fitted.length / 0.5).ceil().clamp(1.0, 1024.0) as usize;
    // Interior samples avoid requesting a nonexistent tile for a boundary endpoint.
    for n in 0..count { tiles.insert(tile_at(c.point_at(fitted.length * (n as f64 + 0.5) / count as f64))); }
    tiles
}

/// Monotone Hermite slopes prevent height overshoot between terrain samples.
fn terrain_parts(s: &MapSpline, heights: &[f64]) -> Vec<MapSpline> {
    let count = heights.len() - 1;
    let step = s.length / count as f64;
    let slopes: Vec<_> = heights.windows(2).map(|h| (h[1] - h[0]) / step).collect();
    let mut tangent = vec![slopes[0]];
    for pair in slopes.windows(2) {
        tangent.push(if pair[0] * pair[1] <= 0.0 { 0.0 } else { 2.0 * pair[0] * pair[1] / (pair[0] + pair[1]) });
    }
    tangent.push(*slopes.last().unwrap());
    let c = curve((0, 0), s);
    (0..count).map(|i| {
        let distance = i as f64 * step;
        let p = c.point_at(distance);
        let mut part = s.clone();
        part.profile_transitions = sliced_transitions(s, distance, step);
        part.pos = [p.x, p.y, heights[i]];
        part.heading = c.heading_at(distance);
        part.length = step;
        part.grad_start = tangent[i] * 100.0;
        part.grad_end = tangent[i + 1] * 100.0;
        part.delta_h = Some(heights[i + 1] - heights[i]);
        part.is_h = true;
        part.cant_start = s.cant_start + (s.cant_end - s.cant_start) * i as f64 / count as f64;
        part.cant_end = s.cant_start + (s.cant_end - s.cant_start) * (i + 1) as f64 / count as f64;
        part.skew_start = s.skew_start + (s.skew_end - s.skew_start) * i as f64 / count as f64;
        part.skew_end = s.skew_start + (s.skew_end - s.skew_start) * (i + 1) as f64 / count as f64;
        part.tex_offset = s.tex_offset + distance;
        part.map_chain_offset = Some(part.tex_offset);
        part
    }).collect()
}

fn sliced_transitions(s: &MapSpline, from: f64, length: f64) -> [Option<omsi_map::ProfileTransition>; 2] {
    s.profile_transitions.map(|t| t.and_then(|mut t| {
        t.station -= from;
        (t.station + t.span > 0.0 && t.station - t.span < length).then_some(t)
    }))
}

fn curve_part(s: &MapSpline, from: f64, length: f64) -> MapSpline {
    let c = curve((0, 0), s);
    let p = c.point_at(from);
    let mut part = s.clone();
    part.profile_transitions = sliced_transitions(s, from, length);
    part.pos = [p.x, p.y, p.z];
    part.heading = c.heading_at(from);
    part.length = length;
    part.grad_start = c.slope_at(from) * 100.0;
    part.grad_end = c.slope_at(from + length) * 100.0;
    part.delta_h = Some(c.height_at(from + length) - p.z);
    part.is_h = true;
    part.cant_start = c.cant_at(from);
    part.cant_end = c.cant_at(from + length);
    part.skew_start = s.skew_start + (s.skew_end - s.skew_start) * from / s.length;
    part.skew_end = s.skew_start + (s.skew_end - s.skew_start) * (from + length) / s.length;
    part.tex_offset = s.tex_offset + from;
    part.map_chain_offset = Some(part.tex_offset);
    part
}

fn between(file: &str, id: i64, tile: (i32, i32), start: DVec3, end: DVec3, offset: f64) -> MapSpline {
    let d = end - start;
    let length = d.truncate().length();
    let grade = d.z / length.max(0.0001) * 100.0;
    let o = origin(tile);
    MapSpline { file: file.into(), id, pos: [start.x - o.x, start.y - o.y, start.z],
        heading: d.x.atan2(d.y).to_degrees().rem_euclid(360.0), length,
        grad_start: grade, grad_end: grade, delta_h: Some(d.z), is_h: true,
        tex_offset: offset, map_chain_offset: Some(offset), ..Default::default() }
}

fn valid_spline(s: &MapSpline) -> bool {
    if s.profile_transitions.iter().flatten().any(|t| !t.valid()) { return false; }
    if !s.length.is_finite() || s.length <= 0.0 || !s.heading.is_finite()
        || !s.pos.iter().all(|n| n.is_finite()) || !s.radius.is_finite()
        || !s.grad_start.is_finite() || !s.grad_end.is_finite()
        || s.delta_h.is_some_and(|h| !h.is_finite()) { return false; }
    [s.cant_start, s.cant_end, s.skew_start, s.skew_end, s.tex_offset].iter().all(|n| n.is_finite())
}

fn inside_tile(s: &MapSpline) -> bool {
    if !valid_spline(s) { return false; }
    let c = SplineCurve::from_map(s, DVec2::ZERO);
    let size = omsi_map::tile_size();
    (0..=128).all(|i| { let p = c.point_at(s.length * i as f64 / 128.0); p.x >= -1e-6 && p.y >= -1e-6 && p.x <= size + 1e-6 && p.y <= size + 1e-6 })
}

fn number(v: f64) -> String { format!("{v:.12}") }

fn record(s: &MapSpline, detail: &str, eol: &str) -> String {
    let mut lines = vec![if s.is_h { "[spline_h]" } else { "[spline]" }.into(), detail.into(),
        s.file.clone(), s.id.to_string(), s.prev_id.to_string(), s.next_id.to_string(),
        number(s.pos[0]), number(s.pos[2]), number(s.pos[1]), number(s.heading), number(s.length),
        number(s.radius), number(s.grad_start), number(s.grad_end)];
    if s.is_h { lines.push(number(s.delta_h.unwrap_or(0.0))); }
    lines.extend([s.cant_start, s.cant_end, s.skew_start, s.skew_end, s.tex_offset].map(number));
    if s.mirror { lines.push("mirror".into()); }
    if s.deleted { lines.push("[openomsi_editor_deleted]".into()); }
    for (end, transition) in s.profile_transitions.iter().enumerate() {
        if let Some(t) = transition {
            lines.extend(["[openomsi_profile_transition]".into(), end.to_string(), number(t.station), number(t.span)]);
            lines.extend(t.x.iter().chain(t.offsets.iter().flatten()).map(|v| number(*v)));
        }
    }
    lines.join(eol) + eol
}

fn extras(s: &MapSpline, eol: &str) -> String {
    let mut lines = Vec::new();
    if s.terrain_align_flag { lines.push("[spline_terrain_align]".to_string()); }
    if let Some(reach) = s.terrain_align { lines.extend(["[spline_terrain_align_2]".into(), number(reach)]); }
    for r in &s.rules {
        lines.extend([if r.kill { "[kill_rule]" } else { "[rule]" }.into(), r.path_index.to_string(),
            r.kind.clone(), number(r.value), number(r.extra)]);
    }
    if lines.is_empty() { String::new() } else { lines.join(eol) + eol }
}

/// Replace absolute spline values, preserving order, rules, unrelated text and line
/// endings. Append new IDs only once: saving twice produces the same file.
pub fn rewrite(text: &str, edits: &HashMap<i64, MapSpline>, added: &[i64]) -> Result<(String, usize), String> {
    if edits.is_empty() { return Ok((text.into(), 0)); }
    let cfg = omsi_cfg::CfgFile::from_str("editor.map", text);
    let parsed = Tile::parse(&cfg);
    if parsed.version != 0 && parsed.version < 14 { return Err("Spline editing needs a version 14 tile".into()); }
    for id in edits.keys() {
        let count = parsed.splines.iter().filter(|s| s.id == *id).count();
        if count > 1 || (count == 0 && !added.contains(id)) { return Err(format!("Spline {id} missing or duplicated; nothing written")); }
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    fn body(l: &str) -> &str { l.trim_end_matches(['\r', '\n']) }
    let default_eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(text.len());
    let mut found = std::collections::HashSet::new();
    let mut i = 0;
    while i < lines.len() {
        let h = body(lines[i]).trim();
        if h.eq_ignore_ascii_case("[object]") {
            if let Some(end) = crate::editor::object_record_end(&lines, i) {
                for line in &lines[i..end] { out.push_str(line); } i = end; continue;
            }
        }
        let is_h = h.eq_ignore_ascii_case("[spline_h]");
        let id = if is_h || h.eq_ignore_ascii_case("[spline]") {
            lines.get(i + 3).and_then(|l| body(l).trim().parse::<i64>().ok())
        } else { None };
        let Some(s) = id.and_then(|id| edits.get(&id)) else { out.push_str(lines[i]); i += 1; continue };
        let n = if is_h { 20 } else { 19 };
        if i + n > lines.len() { return Err(format!("Spline {} has an incomplete record", s.id)); }
        if (4..n).any(|k| body(lines[i + k]).trim().replace(',', ".").parse::<f64>().is_err()) {
            return Err(format!("Spline {} has an incomplete numeric record", s.id));
        }
        let detail = body(lines[i + 1]);
        let eol = &lines[i][body(lines[i]).len()..];
        out.push_str(&record(s, detail, if eol.is_empty() { default_eol } else { eol }));
        found.insert(s.id);
        i += n;
        if lines.get(i).is_some_and(|l| body(l).trim().eq_ignore_ascii_case("mirror")) { i += 1; }
        if lines.get(i).is_some_and(|l| body(l).trim().eq_ignore_ascii_case("[openomsi_editor_deleted]")) { i += 1; }
        while lines.get(i).is_some_and(|l| body(l).trim().eq_ignore_ascii_case("[openomsi_profile_transition]")) {
            if i + 12 > lines.len() || (1..=11).any(|n| body(lines[i + n]).trim().parse::<f64>().is_err()) {
                return Err("Invalid saved profile transition; nothing written".into());
            }
            i += 12;
        }
    }
    let mut new: Vec<_> = edits.values().filter(|s| !found.contains(&s.id)).collect();
    new.sort_by_key(|s| s.id);
    for s in new {
        if !added.contains(&s.id) { return Err(format!("Spline {} record not found", s.id)); }
        if !out.ends_with('\n') { out.push_str(default_eol); }
        out.push_str(default_eol);
        out.push_str(&record(s, "0", default_eol));
        out.push_str(&extras(s, default_eol));
        found.insert(s.id);
    }
    Ok((out, found.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Tile {
        Tile::parse(&omsi_cfg::CfgFile::from_str("test.map", text))
    }

    #[test]
    fn second_connection_pick_releases_the_cache_before_preview_reads_it() {
        let source_key = ((0, 0), 1);
        let target_key = ((0, 0), 2);
        let source = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 0.0), DVec3::new(100.0, 110.0, 0.0), 0.0);
        let target = between("road.sli", 2, (0, 0), DVec3::new(100.0, 111.0, 0.0), DVec3::new(100.0, 120.0, 0.0), 0.0);
        let cache = parking_lot::Mutex::new(Edits { originals: HashMap::from([(source_key, source), (target_key, target)]), ..Default::default() });
        let mut editor = SplineEditor::default();
        assert_eq!(editor.pick_loaded(&cache, &[(0, 0)], DVec3::new(100.0, 100.0, 10.0), -Vec3::Z, &HashMap::new()), Some(1));
        assert!(cache.try_lock().is_some());
        editor.connection = Some(Connection { source: source_key, target: None, preview: None, error: None, replace_links: false, occupied: false, allow_transition: false });
        assert_eq!(editor.pick_loaded(&cache, &[(0, 0)], DVec3::new(100.0, 120.0, 10.0), -Vec3::Z, &HashMap::new()), Some(2));
        assert_eq!(editor.connection.as_ref().unwrap().source, source_key);
        // try_lock makes a lock-lifetime regression fail immediately rather than
        // hanging the test runner, just as the old second-click path hung the UI.
        let (source, target) = {
            let edits = cache.try_lock().expect("second pick retained the spline cache lock");
            (edits.current(source_key).unwrap(), edits.current(target_key).unwrap())
        };
        assert!(connection_geometry((0, 0), &source, (0, 0), &target).is_ok());
        assert!(cache.try_lock().is_some());
    }

    #[test]
    fn generated_spline_roundtrips_axes_and_height() {
        let start = DVec3::new(223.5745, 269.2551, 42.26);
        let end = DVec3::new(240.0, 255.0, 41.36);
        let s = between("Splines\\road.sli", 9852695, (0, 0), start, end, 0.0);
        let tile = parsed(&record(&s, "0", "\r\n"));
        assert_eq!(tile.splines.len(), 1);
        let r = &tile.splines[0];
        assert!((curve((0, 0), r).end_point() - end).length() < 1e-8);
        assert_eq!(r.pos, s.pos);
        assert!((r.delta_h.unwrap() + 0.9).abs() < 1e-10);
        assert_eq!(record(&s, "0", "\n").lines().count(), 20);
    }

    #[test]
    fn generated_ai_rules_survive_map_save_and_reload() {
        let settings=crate::traffic_editor::Settings {forward_bus:true,..Default::default()};
        let source=MapSpline {id:1,length:20.0,cant_start:3.0,cant_end:7.0,skew_start:0.2,skew_end:-0.1,profile_transitions:[None,Some(omsi_map::ProfileTransition {station:20.0,span:5.0,x:[-4.0,4.0],offsets:[[0.5,-0.05,-0.1],[-0.5,0.05,-0.1]]})],..Default::default()};
        let mut plan=crate::traffic_editor::build(&[(((0,0),1),source,false)],&settings).unwrap();
        let s=&mut plan.pieces[0].1;s.id=2;
        let edits=HashMap::from([(2,s.clone())]);
        let (text,n)=rewrite("[version]\n14\n",&edits,&[2]).unwrap();assert_eq!(n,1);
        let tile=parsed(&text);assert_eq!(tile.splines.len(),1);
        assert_eq!(tile.splines[0].rules,s.rules);
        assert_eq!(tile.splines[0].cant_start,s.cant_start);assert_eq!(tile.splines[0].cant_end,s.cant_end);
        assert_eq!(tile.splines[0].skew_start,s.skew_start);assert_eq!(tile.splines[0].skew_end,s.skew_end);
        assert_eq!(tile.splines[0].profile_transitions,s.profile_transitions);
        assert_eq!(rewrite(&text,&edits,&[2]).unwrap().0,text);
    }

    #[test]
    fn rewrites_preserve_order_rules_and_are_idempotent() {
        let a = between("Splines\\road.sli", 1, (0, 0), DVec3::new(10.0, 20.0, 5.0), DVec3::new(10.0, 50.0, 6.0), 0.0);
        let mut b = a.clone(); b.id = 2; b.is_h = false; b.delta_h = None;
        let text = format!("[version]\r\n14\r\n\r\n{}\r\n[rule]\r\n0\r\nspeedlimit\r\n30\r\n0\r\n\r\n{}\r\n[terrain]\r\n", record(&a, "2", "\r\n"), record(&b, "0", "\r\n"));
        let mut changed = a.clone(); changed.pos[0] += 1.0;
        let mut new = a.clone(); new.id = 3;
        let edits = HashMap::from([(1, changed), (3, new)]);
        let (out, n) = rewrite(&text, &edits, &[3]).unwrap();
        assert_eq!(n, 2);
        assert!(out.contains("[rule]\r\n0\r\nspeedlimit\r\n30\r\n0\r\n"));
        assert!(out.contains(&record(&b, "0", "\r\n")));
        let t = parsed(&out);
        assert_eq!(t.splines.iter().map(|s| s.id).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(t.splines[0].rules.len(), 1);
        assert_eq!(rewrite(&out, &edits, &[3]).unwrap().0, out);
    }

    #[test]
    fn missing_ids_and_old_tiles_are_rejected() {
        let s = between("Splines\\road.sli", 9, (0, 0), DVec3::new(1.0, 1.0, 0.0), DVec3::new(1.0, 5.0, 0.0), 0.0);
        let e = HashMap::from([(9, s)]);
        assert!(rewrite("[version]\n14\n", &e, &[]).is_err());
        assert!(rewrite("[version]\n11\n", &e, &[9]).is_err());
        let (out, _) = rewrite("[version]\n14\n", &e, &[9]).unwrap();
        assert_eq!(parsed(&out).splines[0].id, 9);
    }

    #[test]
    fn overlay_keeps_attachment_indices_and_never_appends_twice() {
        let s = between("Splines\\road.sli", 9, (0, 0), DVec3::new(1.0, 1.0, 0.0), DVec3::new(1.0, 5.0, 0.0), 0.0);
        let mut e = Edits::default();
        e.added.insert(((0, 0), 9), s);
        let mut t = Tile::default();
        e.overlay((0, 0), &mut t); e.overlay((0, 0), &mut t);
        assert_eq!(t.splines.len(), 1);
    }

    #[test]
    fn deletion_and_restoration_keep_indices_rules_and_line_endings() {
        let mut a = between("road.sli", 11, (0, 0), DVec3::new(10.0, 10.0, 2.0), DVec3::new(10.0, 40.0, 3.0), 0.0);
        let mut b = a.clone(); b.id = 12;
        let text = format!("[version]\r\n14\r\n{}[rule]\r\n0\r\nspeedlimit\r\n30\r\n0\r\n{}[splineAttachement]\r\n0\r\nlamp.sco\r\n50\r\n1\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n10\r\n100\r\n0\r\n0\r\n", record(&a, "0", "\r\n"), record(&b, "0", "\r\n"));
        a.deleted = true;
        let edits = HashMap::from([(11, a.clone())]);
        let deleted = rewrite(&text, &edits, &[]).unwrap().0;
        let tile = parsed(&deleted);
        assert_eq!(tile.splines.iter().map(|s| s.id).collect::<Vec<_>>(), vec![11, 12]);
        assert!(tile.splines[0].deleted && !tile.splines[1].deleted);
        assert_eq!(tile.splines[0].rules.len(), 1);
        assert_eq!(tile.spline_attachments[0].spline_index, 1);
        assert_eq!(rewrite(&deleted, &edits, &[]).unwrap().0, deleted);
        a.deleted = false;
        assert_eq!(rewrite(&deleted, &HashMap::from([(11, a)]), &[]).unwrap().0, text);
    }

    #[test]
    fn subdividing_a_curve_preserves_its_shape_height_and_texture_distance() {
        let mut s = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 2.0), DVec3::new(100.0, 160.0, 8.0), 12.0);
        s.radius = -80.0; s.grad_start = 3.0; s.grad_end = 7.0;
        let c = curve((0, 0), &s);
        let part = curve_part(&s, 23.0, 37.0);
        let p = curve((0, 0), &part);
        for i in 0..=20 {
            let d = part.length * i as f64 / 20.0;
            assert!((p.point_at(d) - c.point_at(d + 23.0)).length() < 1e-8);
            assert!((p.slope_at(d) - c.slope_at(d + 23.0)).abs() < 1e-10);
        }
        assert_eq!(part.tex_offset, 35.0);
    }

    #[test]
    fn terrain_fit_follows_a_hill_without_overshooting_between_samples() {
        let mut s = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 0.0), DVec3::new(100.0, 120.0, 0.0), 0.0);
        s.radius = 60.0;
        let heights = [0.05, 1.05, 2.05, 1.05, 0.05];
        let parts = terrain_parts(&s, &heights);
        let original = curve((0, 0), &s);
        for (i, part) in parts.iter().enumerate() {
            let c = curve((0, 0), part);
            assert!((c.start.z - heights[i]).abs() < 1e-10);
            assert!((c.end_point().z - heights[i + 1]).abs() < 1e-10);
            for n in 0..=20 {
                let d = part.length * n as f64 / 20.0;
                let p = c.point_at(d);
                assert!(p.z >= heights[i].min(heights[i + 1]) - 1e-9 && p.z <= heights[i].max(heights[i + 1]) + 1e-9);
                assert!((p.truncate() - original.point_at(i as f64 * 5.0 + d).truncate()).length() < 1e-8);
            }
        }
    }

    #[test]
    fn snap_arc_hits_targets_on_both_sides_and_on_a_straight() {
        let s = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 2.0), DVec3::new(100.0, 110.0, 2.0), 0.0);
        for target in [DVec3::new(110.0, 140.0, 7.0), DVec3::new(90.0, 140.0, -1.0), DVec3::new(100.0, 140.0, 2.0)] {
            let snapped = arc_to(&s, (0, 0), target).unwrap();
            assert!((curve((0, 0), &snapped).end_point() - target).length() < 1e-8);
            assert_eq!(snapped.pos, s.pos);
            assert_eq!(snapped.heading, s.heading);
        }
    }

    #[test]
    fn generated_lines_split_at_grid_edges_in_negative_coordinates() {
        let size = omsi_map::tile_size();
        let start = DVec3::new(-size - 10.0, 50.0, 1.0);
        let end = DVec3::new(-size + 30.0, 50.0, 3.0);
        let cuts = straight_tile_cuts(start, end);
        assert_eq!(cuts, vec![0.0, 0.25, 1.0]);
        for pair in cuts.windows(2) {
            let a = start.lerp(end, pair[0]);
            let b = start.lerp(end, pair[1]);
            let tile = tile_at((a + b) * 0.5);
            let part = between("road.sli", 1, tile, a, b, 0.0);
            assert!(inside_tile(&part));
            assert!((curve(tile, &part).end_point() - b).length() < 1e-8);
        }
    }

    #[test]
    fn connects_the_start_of_a_short_spline_across_a_tile_edge() {
        let size = omsi_map::tile_size();
        let source_tile = (1, 0);
        let target_tile = (0, 0);
        let source = between("terrain.sli", 239, source_tile, DVec3::new(size + 0.4, 80.0, 0.05), DVec3::new(size + 7.34, 80.0, 0.05), 0.0);
        let target = between("terrain.sli", 238, target_tile, DVec3::new(size - 20.0, 80.0, 0.05), DVec3::new(size - 0.2, 80.0, 0.05), 10.0);
        let joined = connection_geometry(source_tile, &source, target_tile, &target).unwrap();
        assert_eq!((joined.source_end, joined.target_end), (End::Start, End::Finish));
        assert!((curve(source_tile, &joined.source).start - curve(target_tile, &target).end_point()).length() < 1e-8);
        assert!((curve(source_tile, &joined.source).end_point() - curve(source_tile, &source).end_point()).length() < 1e-8);
        assert!((joined.source.length - 7.54).abs() < 1e-8);
        assert!(joined.source.pos[0] < 0.0 && !inside_tile(&joined.source));
        assert_eq!((joined.source.prev_id, joined.target.next_id), (238, 239));
        assert_eq!(joined.source.id, source.id);
        let tiles = connection_tiles((source_tile, source.id), (target_tile, target.id), &joined.source);
        assert!(tiles.contains(&source_tile) && tiles.contains(&target_tile));
        // Save the out-of-tile start without moving its record or attachment index.
        let base = format!("[version]\n14\n{}", record(&source, "0", "\n"));
        let edits = HashMap::from([(source.id, joined.source.clone())]);
        let written = rewrite(&base, &edits, &[]).unwrap().0;
        let read = parsed(&written);
        assert_eq!(read.splines.len(), 1);
        assert!((curve(source_tile, &read.splines[0]).start - curve(target_tile, &target).end_point()).length() < 1e-8);
        assert_eq!(rewrite(&written, &edits, &[]).unwrap().0, written);
    }

    #[test]
    fn connects_all_endpoint_orientations_and_preserves_the_far_anchor() {
        let source = between("road.sli", 1, (0, 0), DVec3::new(50.0, 50.0, 2.0), DVec3::new(50.0, 60.0, 2.2), 0.0);
        let targets = [
            (End::Finish, End::Start, DVec3::new(50.0, 60.3, 2.3), DVec3::new(50.0, 80.0, 3.0)),
            (End::Finish, End::Finish, DVec3::new(50.0, 80.0, 3.0), DVec3::new(50.0, 60.3, 2.3)),
            (End::Start, End::Finish, DVec3::new(50.0, 30.0, 1.0), DVec3::new(50.0, 49.7, 1.8)),
            (End::Start, End::Start, DVec3::new(50.0, 49.7, 1.8), DVec3::new(50.0, 30.0, 1.0)),
        ];
        for (source_end, target_end, from, to) in targets {
            let target = between("road.sli", 2, (0, 0), from, to, 10.0);
            let joined = connection_geometry((0, 0), &source, (0, 0), &target).unwrap();
            assert_eq!((joined.source_end, joined.target_end), (source_end, target_end));
            let c = curve((0, 0), &joined.source);
            assert!((source_end.point(&c) - target_end.point(&curve((0, 0), &target))).length() < 1e-8);
            let fixed = if source_end == End::Start { End::Finish } else { End::Start };
            assert!((fixed.point(&c) - fixed.point(&curve((0, 0), &source))).length() < 1e-8);
            assert_eq!(source_end.link(&joined.source), 2);
            assert_eq!(target_end.link(&joined.target), 1);
            assert_eq!(joined.target.pos, target.pos);
            assert_eq!(joined.target.heading, target.heading);
        }
    }

    #[test]
    fn connection_rejects_occupied_ends_and_bad_direction_without_changing_records() {
        let mut source = between("road.sli", 1, (0, 0), DVec3::new(50.0, 50.0, 0.0), DVec3::new(50.0, 60.0, 0.0), 0.0);
        let target = between("road.sli", 2, (0, 0), DVec3::new(50.0, 60.3, 0.0), DVec3::new(50.0, 80.0, 0.0), 0.0);
        source.next_id = 99;
        assert!(connection_geometry((0, 0), &source, (0, 0), &target).unwrap_err().contains("occupied"));
        source.next_id = 0;
        let side = between("road.sli", 3, (0, 0), DVec3::new(50.0, 60.3, 0.0), DVec3::new(70.0, 60.3, 0.0), 0.0);
        assert!(connection_geometry((0, 0), &source, (0, 0), &side).unwrap_err().contains("End directions"));
        assert_eq!(source.next_id, 0);
        assert_eq!(target.prev_id, 0);
    }

    fn road_patch(x0: f64, x1: f64, y0: f64, y1: f64, height: f64, grade: f64, cant: f64) -> Roadbed {
        let z = |x: f64, y: f64| height + (y - y0) * grade + (x - x0) * cant;
        let mesh = omsi_geometry::MeshData {
            positions: [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| DVec3::new(x, y, z(x, y)).as_vec3()).to_vec(),
            indices: vec![0, 1, 2, 0, 2, 3], ..Default::default()
        };
        let mut bed = Roadbed::default(); bed.add_mesh(&mesh, DVec3::ZERO); bed
    }

    #[test]
    fn smoothing_clears_both_road_edges_and_leaves_distant_ground_unchanged() {
        let bed = road_patch(142.0, 150.0, 50.0, 90.0, 0.2, 0.015, -0.02);
        let mut terrain = omsi_map::Terrain::flat(); terrain.heights.fill(6.0);
        let original = HashMap::from([((0, 0), terrain.clone())]);
        let guard = omsi_map::tile_size() / terrain.cells as f64 * std::f64::consts::SQRT_2;
        let plan = terrain_plan(&bed, &original, guard, 2.0);
        assert_eq!(plan.after.len(), 1);
        let t = &plan.after[0].1;
        for x in [142.001, 145.0, 149.999] {
            for y in [50.01, 55.0, 69.3, 89.99] {
                let road = 0.2 + (y - 50.0) * 0.015 - (x - 142.0) * 0.02;
                assert!(t.sample(x as f32, y as f32) as f64 <= road - ROAD_CLEARANCE + 0.001);
            }
        }
        assert_eq!(t.height_at(0, 0), terrain.height_at(0, 0));
        assert_eq!(original[&(0, 0)], terrain);
        assert_eq!(plan.before[0].1, terrain);
    }

    #[test]
    fn fitting_a_raised_profile_does_not_add_its_height_on_every_fit() {
        let profile = road_patch(140.0, 150.0, 50.0, 90.0, 1.2, 0.0, 0.0);
        let fitted = fitted_height(0.15, DVec3::new(145.0, 70.0, 1.0), &profile);
        assert!(fitted.abs() < 1e-6);
        let after = road_patch(140.0, 150.0, 50.0, 90.0, fitted + 0.2, 0.0, 0.0);
        let repeated = fitted_height(0.15, DVec3::new(145.0, 70.0, fitted), &after);
        assert!((repeated - fitted).abs() < 1e-6);
    }

    #[test]
    fn narrow_road_clears_terrain_triangles_even_without_a_vertex_inside_its_width() {
        let bed = road_patch(147.0, 148.0, 51.0, 89.0, 0.2, 0.0, 0.0);
        let mut terrain = omsi_map::Terrain::flat(); terrain.heights.fill(8.0);
        let plan = terrain_plan(&bed, &HashMap::from([((0, 0), terrain)]), 0.0, 2.0);
        let t = &plan.after[0].1;
        for x in [147.001, 147.5, 147.999] {
            for y in [51.001, 52.4, 62.7, 75.9, 88.999] {
                assert!(t.sample(x, y) <= 0.151, "ground still covers the road at {x}/{y}");
            }
        }
    }

    #[test]
    fn road_smoothing_matches_changed_nodes_on_both_sides_of_a_tile_border() {
        let size = omsi_map::tile_size();
        let bed = road_patch(size - 8.0, size + 8.0, 50.0, 90.0, 0.2, 0.01, 0.0);
        let mut a = omsi_map::Terrain::flat(); a.heights.fill(5.0);
        let mut b = a.clone(); b.heights.fill(6.0);
        let originals = HashMap::from([((0, 0), a.clone()), ((1, 0), b.clone())]);
        let plan = terrain_plan(&bed, &originals, size / a.cells as f64 * std::f64::consts::SQRT_2, 2.0);
        let changed: HashMap<_, _> = plan.after.into_iter().collect();
        for iy in 0..a.samples() {
            let left = changed[&(0, 0)].height_at(a.cells, iy);
            let right = changed[&(1, 0)].height_at(0, iy);
            if left != a.height_at(a.cells, iy) || right != b.height_at(0, iy) { assert_eq!(left, right); }
        }
        assert_eq!(changed[&(0, 0)].height_at(0, 0), 5.0);
        assert_eq!(changed[&(1, 0)].height_at(b.cells, 0), 6.0);
    }

    #[test]
    fn smoothing_undo_restores_original_terrain_even_after_edited_bytes_were_saved() {
        let bed = road_patch(142.0, 150.0, 50.0, 90.0, 0.2, 0.0, 0.0);
        let mut t = omsi_map::Terrain::flat(); t.heights.fill(6.0);
        let plan = terrain_plan(&bed, &HashMap::from([((0, 0), t.clone())]), 8.0, 2.0);
        let mut editor = SplineEditor::default(); editor.remember_with_terrain(Vec::new(), plan.before);
        let mut saved: HashMap<_, _> = plan.after.into_iter().map(|(key, t)| (key, omsi_map::Terrain::parse(&t.to_bytes()).unwrap())).collect();
        assert_ne!(saved[&(0, 0)], t);
        editor.undo.pop().unwrap().restore_terrain(&mut saved);
        assert_eq!(saved[&(0, 0)], t);
        assert_eq!(omsi_map::Terrain::parse(&saved[&(0, 0)].to_bytes()).unwrap(), t);
    }

    fn replacement_fixture() -> Edits {
        let mut source = between("road.sli", 1, (0, 0), DVec3::new(50.0, 50.0, 0.0), DVec3::new(50.0, 60.0, 0.0), 0.0);
        let mut target = between("road.sli", 2, (0, 0), DVec3::new(50.0, 60.3, 0.0), DVec3::new(50.0, 80.0, 0.0), 0.0);
        source.prev_id = 8; source.next_id = 9;
        target.prev_id = 10; target.next_id = 11;
        let mut old_source_neighbor = source.clone(); old_source_neighbor.id = 9;
        old_source_neighbor.prev_id = 1; old_source_neighbor.next_id = 12;
        let mut old_target_neighbor = target.clone(); old_target_neighbor.id = 10;
        old_target_neighbor.prev_id = 13; old_target_neighbor.next_id = 2;
        Edits { originals: HashMap::from([
            (((0, 0), 1), source), (((0, 0), 2), target),
            (((0, 0), 9), old_source_neighbor), (((0, 0), 10), old_target_neighbor),
        ]), ..Default::default() }
    }

    #[test]
    fn replacement_previews_both_reciprocal_links_without_mutating_the_map() {
        let edits = replacement_fixture();
        assert!(connection_plan(&edits, &[(0, 0)], None, ((0, 0), 1), ((0, 0), 2), false, false).unwrap_err().contains("occupied by 9"));
        let preview = connection_plan(&edits, &[(0, 0)], None, ((0, 0), 1), ((0, 0), 2), true, false).unwrap();
        assert_eq!((preview.source.prev_id, preview.source.next_id), (8, 2));
        assert_eq!((preview.target.prev_id, preview.target.next_id), (1, 11));
        assert_eq!(preview.neighbors.len(), 2);
        assert_eq!((preview.neighbors[0].1.prev_id, preview.neighbors[0].1.next_id), (0, 12));
        assert_eq!((preview.neighbors[1].1.prev_id, preview.neighbors[1].1.next_id), (13, 0));
        assert_eq!(edits.current(((0, 0), 1)).unwrap().next_id, 9);
        assert_eq!(edits.current(((0, 0), 9)).unwrap().prev_id, 1);
        assert!(edits.changed.is_empty());
        let mut changed = HashMap::from([(1, preview.source), (2, preview.target)]);
        for (_, neighbor) in preview.neighbors { changed.insert(neighbor.id, neighbor); }
        let original = format!("[version]\n14\n{}", [1, 2, 9, 10].map(|id| record(&edits.current(((0, 0), id)).unwrap(), "0", "\n")).join("\n"));
        let saved = rewrite(&original, &changed, &[]).unwrap().0;
        let tile = parsed(&saved);
        assert_eq!(tile.splines.iter().map(|s| (s.id, s.prev_id, s.next_id)).collect::<Vec<_>>(),
            vec![(1, 8, 2), (2, 1, 11), (9, 0, 12), (10, 13, 0)]);
    }

    #[test]
    fn replacement_distinguishes_missing_neighbors_from_unloaded_or_unread_tiles() {
        let mut edits = replacement_fixture();
        edits.originals.get_mut(&((0, 0), 1)).unwrap().next_id = 99;
        edits.originals.get_mut(&((0, 0), 2)).unwrap().prev_id = 0;
        let mut index = crate::tiles::MapIndex::default();
        index.splines.insert(99, crate::tiles::IndexedSpline { length: 10.0, map_chain_offset: None, prev: 1, next: 0 });
        assert!(connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap_err().contains("99 for editing first"));
        index.splines.remove(&99);
        let preview = connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap();
        assert_eq!(preview.source.next_id, 2);
        assert!(preview.neighbors.is_empty());
        index.tiles_failed = 1;
        assert!(connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap_err().contains("not fully"));
        let mut neighbor = edits.current(((0, 0), 9)).unwrap(); neighbor.id = 99;
        edits.originals.insert(((1, 0), 99), neighbor);
        assert!(connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap_err().contains("99 first"));
        assert_eq!(edits.current(((0, 0), 1)).unwrap().next_id, 99);
    }

    #[test]
    fn replacement_merges_two_detachments_from_the_same_old_neighbor() {
        let mut edits = replacement_fixture();
        edits.originals.get_mut(&((0, 0), 2)).unwrap().prev_id = 9;
        edits.originals.get_mut(&((0, 0), 9)).unwrap().next_id = 2;
        let preview = connection_plan(&edits, &[(0, 0)], None, ((0, 0), 1), ((0, 0), 2), true, false).unwrap();
        assert_eq!(preview.neighbors.len(), 1);
        assert_eq!((preview.neighbors[0].1.prev_id, preview.neighbors[0].1.next_id), (0, 0));
        let old = edits.current(((0, 0), 9)).unwrap();
        assert_eq!((old.prev_id, old.next_id), (1, 2));
    }
}

#[cfg(test)]
mod edge_connector_tests {
    use super::*;

    fn profile(left: f32, right: f32) -> omsi_scenery::Spline {
        omsi_scenery::Spline { profiles: vec![omsi_scenery::sli::SplineProfile {
            texture: 0, points: vec![
                omsi_scenery::sli::SplineProfilePoint { x: left, z: 0.2, ..Default::default() },
                omsi_scenery::sli::SplineProfilePoint { x: right, z: 0.2, ..Default::default() },
            ],
        }], ..Default::default() }
    }

    fn roads() -> (MapSpline, MapSpline) {
        (between("a.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 2.0), DVec3::new(100.0, 140.0, 2.0), 0.0),
         between("b.sli", 2, (0, 0), DVec3::new(100.0, 140.5, 2.0), DVec3::new(100.0, 180.0, 2.0), 0.0))
    }

    #[test]
    fn unequal_widths_require_opt_in_and_roundtrip_without_duplicate_metadata() {
        let (a, b) = roads();
        let (ad, bd) = (profile(-3.0, 3.0), profile(-4.0, 4.0));
        let mut p = connection_geometry((0, 0), &a, (0, 0), &b).unwrap();
        assert!(align_connection_edges((0, 0), &a, &ad, (0, 0), &bd, &mut p, false).unwrap_err().contains("optional"));
        align_connection_edges((0, 0), &a, &ad, (0, 0), &bd, &mut p, true).unwrap();
        let got = edge_points((0, 0), &p.source, &ad, End::Finish);
        let want = edge_points((0, 0), &p.target, &bd, End::Start);
        for i in 0..2 { assert!(got[i].distance(want[i]) < 1e-8); }
        assert_eq!(a.profile_transitions, [None; 2]);
        assert_eq!(p.target.profile_transitions, [None; 2]);
        let text = format!("[version]\n14\n{}{}", record(&a, "0", "\n"), record(&b, "0", "\n"));
        let changes = HashMap::from([(a.id, p.source.clone()), (b.id, p.target.clone())]);
        let out = rewrite(&text, &changes, &[]).unwrap().0;
        let tile = Tile::parse(&omsi_cfg::CfgFile::from_str("test.map", &out));
        assert_eq!(tile.splines[0].profile_transitions, p.source.profile_transitions);
        assert_eq!(rewrite(&out, &changes, &[]).unwrap().0, out);
        assert_eq!(out.matches("[openomsi_profile_transition]").count(), 1);
        let restored = rewrite(&out, &HashMap::from([(a.id, a.clone()), (b.id, b.clone())]), &[]).unwrap().0;
        assert_eq!(restored, text);
        // Reconnecting produces identical geometry instead of accumulating a second taper.
        let mut again = connection_geometry((0, 0), &p.source, (0, 0), &p.target).unwrap();
        align_connection_edges((0, 0), &p.source, &ad, (0, 0), &bd, &mut again, true).unwrap();
        assert_eq!(again.source, p.source);
    }

    #[test]
    fn handles_match_rendered_vertices_with_curve_mirror_cant_and_skew() {
        let (mut a, _) = roads();
        a.radius = 80.0; a.mirror = true; a.skew_start = 0.1; a.skew_end = -0.2;
        a.cant_start = 4.0; a.cant_end = -2.0;
        let def = profile(-2.0, 5.0);
        let tile = (-17, 10);
        let o = origin(tile).extend(0.0);
        let mesh = omsi_geometry::build_spline_mesh(&def, &curve(tile, &a), true, o);
        for end in [End::Start, End::Finish] {
            for p in edge_points(tile, &a, &def, end) {
                assert!(mesh.positions.iter().any(|v| (o + v.as_dvec3()).distance(p) < 0.0001));
            }
        }
    }

    #[test]
    fn small_heading_mismatch_closes_both_edges_and_keeps_far_end() {
        let (a, mut b) = roads(); b.heading = 1.5;
        let def = profile(-3.5, 3.5);
        let mut p = connection_geometry((0, 0), &a, (0, 0), &b).unwrap();
        align_connection_edges((0, 0), &a, &def, (0, 0), &def, &mut p, false).unwrap();
        for (x, y) in edge_points((0, 0), &p.source, &def, End::Finish).into_iter()
            .zip(edge_points((0, 0), &p.target, &def, End::Start)) { assert!(x.distance(y) < 1e-8); }
        assert_eq!(edge_points((0, 0), &a, &def, End::Start), edge_points((0, 0), &p.source, &def, End::Start));
    }

    #[test]
    fn transitions_follow_subdivision_without_moving_the_surface() {
        let (a, b) = roads();
        let (ad, bd) = (profile(-3.0, 3.0), profile(-4.0, 4.0));
        let mut p = connection_geometry((0, 0), &a, (0, 0), &b).unwrap();
        align_connection_edges((0, 0), &a, &ad, (0, 0), &bd, &mut p, true).unwrap();
        let from = p.source.length - 6.0;
        let part = curve_part(&p.source, from, 6.0);
        for n in 0..=12 {
            let u = n as f64 * 0.5;
            for x in [-3.0, 0.0, 3.0] {
                let whole = omsi_geometry::spline_profile_point(&ad, &curve((0, 0), &p.source), false, from + u, x, 0.2);
                let split = omsi_geometry::spline_profile_point(&ad, &curve((0, 0), &part), false, u, x, 0.2);
                assert!(whole.distance(split) < 1e-8);
            }
        }
    }

    #[test]
    fn different_curb_shapes_are_not_silently_connected() {
        let a = profile(-3.0, 3.0);
        let mut b = profile(-4.0, 4.0);
        b.profiles[0].points.insert(1, omsi_scenery::sli::SplineProfilePoint { x: 0.0, z: 0.5, ..Default::default() });
        assert!(!compatible_sections(&a, false, &b, false, false));
        assert!(compatible_sections(&a, true, &profile(-4.0, 4.0), false, true));
    }

    #[test]
    fn reversed_ends_and_tile_boundaries_keep_both_edges_aligned() {
        let def = profile(-3.0, 3.0);
        for (source_end, target_end) in [(End::Start, End::Start), (End::Start, End::Finish),
                                       (End::Finish, End::Start), (End::Finish, End::Finish)] {
            let source_tile = (-1, 0); let target_tile = (0, 0);
            let join = DVec3::new(0.0, 150.0, 2.0);
            let left = join - DVec3::X * 40.0; let right = join + DVec3::X * 40.0;
            let a = if source_end == End::Finish { between("a.sli", 1, source_tile, left, join, 0.0) }
                else { between("a.sli", 1, source_tile, join, left, 0.0) };
            let b = if target_end == End::Start { between("b.sli", 2, target_tile, join, right, 0.0) }
                else { between("b.sli", 2, target_tile, right, join, 0.0) };
            let mut p = connection_geometry(source_tile, &a, target_tile, &b).unwrap();
            align_connection_edges(source_tile, &a, &def, target_tile, &profile(-4.0, 4.0), &mut p, true).unwrap();
            let got = edge_points(source_tile, &p.source, &def, source_end);
            let mut want = edge_points(target_tile, &b, &profile(-4.0, 4.0), target_end);
            if source_end == target_end { want.swap(0, 1); }
            for i in 0..2 { assert!(got[i].distance(want[i]) < 1e-8); }
        }
    }
    #[test]
    fn curved_ends_with_angle_gap_join_both_edges_only_when_enabled() {
        let (mut a,mut b)=roads();a.radius=160.0;a.prev_id=99;
        let endpoint=curve((0,0),&a).end_point();
        b.pos=[endpoint.x+0.1,endpoint.y+0.25,endpoint.z];b.heading=curve((0,0),&a).heading_at(a.length)+6.0;b.radius=-120.0;
        let def=profile(-3.5,3.5);
        assert!(connection_geometry((0,0),&a,(0,0),&b).is_err());
        let mut p=connection_geometry_impl((0,0),&a,(0,0),&b,false,true).unwrap();
        align_connection_edges((0,0),&a,&def,(0,0),&def,&mut p,true).unwrap();
        let want=edge_points((0,0),&b,&def,End::Start);
        for (actual,want) in edge_points((0,0),&p.source,&def,End::Finish).iter().zip(want) {assert!(actual.distance(want)<0.001);}
        for (old,new) in edge_points((0,0),&a,&def,End::Start).iter().zip(edge_points((0,0),&p.source,&def,End::Start)) {assert!(old.distance(new)<0.005);}
        assert_eq!(p.source.prev_id,99);assert_eq!(p.source.next_id,b.id);
        let text=record(&p.source,"0","\n");let restored=Tile::parse(&omsi_cfg::CfgFile::from_str("curve.map",&text));
        for (saved,built) in edge_points((0,0),&restored.splines[0],&def,End::Finish).iter().zip(edge_points((0,0),&p.source,&def,End::Finish)) {assert!(saved.distance(built)<1e-8);}
        let mut again=connection_geometry_impl((0,0),&p.source,(0,0),&p.target,false,true).unwrap();
        align_connection_edges((0,0),&p.source,&def,(0,0),&def,&mut again,true).unwrap();
        for n in 0..=80 {for x in [-3.5,0.0,3.5] {
            let station=p.source.length*n as f64/80.0;
            let before=omsi_geometry::spline_profile_point(&def,&curve((0,0),&p.source),false,station,x,0.2);
            let after=omsi_geometry::spline_profile_point(&def,&curve((0,0),&again.source),false,station,x,0.2);
            assert!(before.distance(after)<1e-8);
        }}
        b.heading+=60.0;assert!(connection_geometry_impl((0,0),&a,(0,0),&b,false,true).is_err());
    }

}

#[cfg(test)]
mod junction_connection_tests {
    use super::*;
    fn section(width:f32)->omsi_scenery::Spline{omsi_scenery::Spline{profiles:vec![omsi_scenery::SplineProfile{texture:0,points:[-width/2.0,width/2.0].into_iter().map(|x|omsi_scenery::SplineProfilePoint{x,z:0.0,u:0.0,v_scale:1.0}).collect()}],..Default::default()}}
    #[test]fn only_unique_explicitly_deleted_neighbour_releases_junction_end(){
        let dead=MapSpline{id:9859291,deleted:true,..Default::default()};
        assert!(deleted_junction_neighbor(9859291,9859279,&[dead.clone()]));
        assert!(!deleted_junction_neighbor(9859291,9859279,&[]));
        assert!(!deleted_junction_neighbor(9859291,9859279,&[dead.clone(),dead.clone()]));
        let mut live=dead.clone();live.deleted=false;assert!(!deleted_junction_neighbor(9859291,9859279,&[live]));
        assert!(!deleted_junction_neighbor(9859279,9859279,&[dead]));
    }
    // Saved road geometry and junction placement from the reported Arm A failure.
    // Flat profile isolates the connection logic from the external Yufa asset.
    #[test]fn saved_arm_a_reconnects_after_deleted_neighbour_is_released(){
        let project:crate::junction_builder::Project=serde_json::from_str(r#"{
  "format": 1,
  "name": "Eigene T-Kreuzung",
  "arms": [
    {
      "enabled": true,
      "angle": 270.0,
      "width": 11.0,
      "length": 30.0,
      "bend": -15.0,
      "sidewalk": 0.0
    },
    {
      "enabled": true,
      "angle": 90.0,
      "width": 11.0,
      "length": 30.0,
      "bend": 20.0,
      "sidewalk": 0.0
    },
    {
      "enabled": true,
      "angle": 210.0,
      "width": 7.0,
      "length": 23.0,
      "bend": 30.0,
      "sidewalk": 0.0
    },
    {
      "enabled": false,
      "angle": 0.0,
      "width": 7.0,
      "length": 30.0,
      "bend": 0.0,
      "sidewalk": 0.0
    }
  ],
  "corner": 5.0,
  "texture_metres": 5.98802410597492,
  "road_surface": {
    "u_start": 0.0,
    "u_end": 1.0,
    "reverse_v": false,
    "width_metres": 4.0
  },
  "road_texture": "Splines/ADDON_Oberpfalz_Streets/texture/asphalt2.bmp",
  "walk_texture": "",
  "markings": true,
  "left_hand": false
}"#).unwrap();
        let source=MapSpline{id:9859290,prev_id:9859289,next_id:9859291,
            pos:[35.615598299740,277.687042946505,-0.232895930861],heading:57.316616969055,
            length:31.790261109734,radius:77.832046833231,grad_start:1.870971417934,grad_end:1.870971417934,
            delta_h:Some(0.594786699050),is_h:true,tex_offset:129.709242215247,..Default::default()};
        let(p,h,def)=crate::junction_builder::port(&project,0).unwrap();let(s,c)=365.0_f64.to_radians().sin_cos();
        let target=DVec3::new(97.5289,289.1343,0.6842)+DVec3::new(p.x*c+p.y*s,p.y*c-p.x*s,p.z);
        let profile=section(11.0);
        assert!(junction_fit((0,0),&source,&profile,9859279,target,h+365.0,&def).is_err());
        let dead=MapSpline{id:9859291,deleted:true,..Default::default()};
        assert!(deleted_junction_neighbor(source.next_id,9859279,&[dead]));
        let mut released=source.clone();released.next_id=0;
        let fitted=junction_fit((0,0),&released,&profile,9859279,target,h+365.0,&def).unwrap();
        assert_eq!(fitted.next_id,9859279);assert_eq!(fitted.prev_id,source.prev_id);
        assert_eq!(fitted.pos,source.pos);assert_eq!(fitted.tex_offset,source.tex_offset);
        assert!(curve((0,0),&fitted).end_point().distance(target)<1e-6);
        let again=junction_fit((0,0),&fitted,&profile,9859279,target,h+365.0,&def).unwrap();
        assert!((again.length-fitted.length).abs()<1e-7);
    }
    #[test]fn both_road_ends_join_rotated_arms_and_preserve_far_end(){
        for start in [false,true]{for heading in [0.0_f64,37.0,180.0,285.0]{
            let point=DVec3::new(100.0,100.0,4.0);let dir=SplineCurve::dir(heading).extend(0.0);
            let p=point+dir*if start{3.0}else{25.0};
            let source=MapSpline{id:7,file:"road.sli".into(),pos:p.to_array(),heading:heading+if start{0.0}else{180.0},length:22.0,tex_offset:3.75,..Default::default()};
            let def=section(7.0);let fitted=junction_fit((0,0),&source,&def,99,point,heading,&def).unwrap();
            let end=if start{End::Start}else{End::Finish};let far=if start{End::Finish}else{End::Start};
            assert_eq!(end.link(&fitted),99);assert_eq!(far.link(&fitted),0);assert_eq!(fitted.id,7);assert_eq!(fitted.file,source.file);
            assert!(end.point(&curve((0,0),&fitted)).distance(point)<1e-6);
            assert!(far.point(&curve((0,0),&fitted)).distance(far.point(&curve((0,0),&source)))<1e-6);
            if !start{assert_eq!(fitted.tex_offset,source.tex_offset);}
            let again=junction_fit((0,0),&fitted,&def,99,point,heading,&def).unwrap();
            assert!((again.length-fitted.length).abs()<1e-7);
        }}
    }
    #[test]fn width_transition_is_checked_and_occupied_end_is_not_overwritten(){
        let source=MapSpline{id:7,file:"road.sli".into(),pos:[100.0,130.0,0.0],heading:180.0,length:28.0,..Default::default()};
        let point=DVec3::new(100.0,100.0,0.0);let def=section(7.0);
        let fitted=junction_fit((0,0),&source,&def,99,point,0.0,&section(10.0)).unwrap();
        let edges=edge_points((0,0),&fitted,&def,End::Finish);assert!((edges[0].distance(edges[1])-10.0).abs()<1e-6);
        let mut occupied=source.clone();occupied.next_id=123;
        assert!(junction_fit((0,0),&occupied,&def,99,point,0.0,&def).is_err());assert_eq!(occupied.next_id,123);
        assert!(junction_fit((0,0),&source,&def,99,point,90.0,&def).is_err());
    }
    #[test]fn disabled_arm_rejected_and_port_matches_curved_export_mesh(){
        let mut project=crate::junction_builder::Project::default();project.arms[0].bend=18.0;
        let built=crate::junction_builder::build(&project).unwrap();let(p,heading,_)=crate::junction_builder::port(&project,0).unwrap();
        assert_eq!(heading,288.0);let d=SplineCurve::dir(heading);let right=DVec3::new(d.y,-d.x,0.0);
        for sign in [-1.0,1.0]{let edge=p+right*(sign*project.arms[0].width/2.0);
            assert!(built.mesh.vertices.iter().any(|v|DVec3::new(v.position.x as f64,v.position.z as f64,v.position.y as f64).distance(edge)<1e-5));}
        assert!(crate::junction_builder::port(&project,3).is_err());
    }
}

#[cfg(test)]
#[path="junction_connections_tests.rs"]
mod junction_pose_connection_tests;
