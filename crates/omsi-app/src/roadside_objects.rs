//! Editable object rows along a road. OMSI attachments remain the rendered/saved objects;
//! a small editor-only recipe preserves spacing when the road geometry changes.
use crate::{scene::World, spline_editor::Key};
use glam::{DVec2, DVec3};
use hashbrown::{HashMap, HashSet};
use omsi_geometry::SplineCurve;
use omsi_map::{MapSpline, SplineAttachment, Tile};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const DEFAULT_OBJECT: &str = "Sceneryobjects\\ADDON_gcW\\leitpfosten.sco";
const META: &str = "[openomsi_editor_object_row]";
const LIMIT: usize = 4000;
type TileKey = (i32, i32);

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub interval: f64,
    pub margin: f64,
    pub start: f64,
    pub range: f64,
    pub height: f64,
    pub rotation: f64,
    pub sides: u8, // bit 1 left, bit 2 right, in the direction of the whole route
    pub connected: bool,
    pub ground: bool,
    pub junction_gap: f64,
    pub manual_gaps: Vec<[f64; 2]>,
}
impl Default for Settings {
    fn default() -> Self {
        Self { interval: 50.0, margin: 0.5, start: 0.0, range: 0.0, height: 0.0,
            rotation: 0.0, sides: 3, connected: false, ground: false, junction_gap: 8.0, manual_gaps: Vec::new() }
    }
}
impl Settings {
    fn validate(&self) -> Result<(), String> {
        for (value, low, high) in [(self.interval, 1.0, 1000.0), (self.margin, 0.0, 30.0),
            (self.start, 0.0, 100000.0), (self.range, 0.0, 100000.0),
            (self.height, -20.0, 20.0), (self.rotation, -360.0, 360.0), (self.junction_gap, 0.0, 100.0)] {
            if !value.is_finite() || !(low..=high).contains(&value) {
                return Err("Abstand oder Höhe liegt außerhalb des erlaubten Bereichs".into());
            }
        }
        if !(1..=3).contains(&self.sides) { return Err("Mindestens eine Straßenseite wählen".into()); }
        for [a, b] in &self.manual_gaps {
            if !a.is_finite() || !b.is_finite() || *a < 0.0 || *b <= *a || *b > 100000.0 {
                return Err("Ungültiger ausgesparter Bereich".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: i64,
    pub start: Key,
    pub file: String,
    pub settings: Settings,
    pub rows: Vec<RowId>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RowId { pub id: i64, pub tile: TileKey, pub spline: i64, pub ordinal: usize, pub side: u8 }
#[derive(Clone, PartialEq)]
pub struct RowEdit { pub spline: i64, pub row: SplineAttachment }
#[derive(Clone)]
struct Source { tile: Arc<Tile>, editable: Arc<HashSet<i64>> }
#[derive(Clone, Default)]
pub struct Edits {
    initialized: bool,
    sources: HashMap<TileKey, Source>,
    pub groups: Vec<Group>,
    pub changes: HashMap<TileKey, HashMap<i64, Option<RowEdit>>>,
    pub dirty_tiles: HashSet<TileKey>,
    pub save_tiles: HashSet<TileKey>,
    undo: Vec<Vec<Group>>,
}
impl Edits {
    pub fn overlay(&self, key: TileKey, tile: &mut Tile) {
        let Some(changes) = self.changes.get(&key) else { return; };
        tile.spline_attachments.retain(|row| !changes.contains_key(&row.id));
        let mut rows: Vec<_> = changes.values().flatten().collect();
        rows.sort_by_key(|edit| edit.row.id);
        for edit in rows {
            if let Some(index) = tile.splines.iter().position(|s| s.id == edit.spline && !s.deleted) {
                let mut row = edit.row.clone(); row.spline_index = index as i32;
                tile.spline_attachments.push(row);
            }
        }
    }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
}

#[derive(Clone)]
pub struct Point {
    pub key: Key, pub ordinal: usize, pub side: u8, pub pos: DVec3,
    pub row: SplineAttachment,
}
#[derive(Clone, Default)]
pub struct Plan { pub points: Vec<Point>, pub length: f64, pub skipped: usize, pub segments: usize, pub gaps: Vec<[f64; 2]> }
#[derive(Clone)]
struct Segment { key: Key, spline: MapSpline, backwards: bool, edges: (f64, f64) }
fn curve(key: Key, spline: &MapSpline) -> SplineCurve {
    let size = omsi_map::tile_size();
    SplineCurve { half_cant_width: omsi_geometry::half_cant_width_of(&spline.file),
        ..SplineCurve::from_map(spline, DVec2::new(key.0.0 as f64 * size, key.0.1 as f64 * size)) }
}
fn same_file(a: &str, b: &str) -> bool { a.replace('\\', "/").eq_ignore_ascii_case(&b.replace('\\', "/")) }

/// Recipes are loaded once. Never hold this lock while reserving IDs or loading assets.
pub fn initialize(world: &World) -> Result<(), String> {
    if world.roadside_edits.lock().initialized { return Ok(()); }
    if world.global.world_coordinates { return Err("Objektreihen brauchen eine normale OMSI-Karte".into()); }
    let mut sources = HashMap::new(); let mut groups = Vec::new();
    for (_, x, y, path) in world.map_tiles() {
        if !omsi_cfg::vfs::is_file(&path) { continue; }
        let (tile, editable) = world.editor_row_source((x, y))?;
        if tile.version != 0 && tile.version < 14 { continue; }
        let bytes = omsi_cfg::vfs::read(&path).map_err(|e| e.to_string())?;
        let text = crate::editor::decode(&bytes).0;
        for group in read_groups(&text)? {
            group.settings.validate()?;
            if group.start.0 != (x, y) { return Err("Objektreihe gehört zu einem anderen Tile".into()); }
            if groups.iter().any(|g: &Group| g.id == group.id) { return Err("Doppelte Kennung einer Objektreihe".into()); }
            groups.push(group);
        }
        sources.insert((x, y), Source { tile: Arc::new(tile), editable: Arc::new(editable) });
    }
    let mut owned = HashSet::new();
    for group in &groups {
        if group.rows.len() > LIMIT { return Err("Gespeicherte Objektreihe hat zu viele Einträge".into()); }
        for row in &group.rows {
            let valid = sources.get(&row.tile).is_some_and(|source| source.tile.spline_attachments.iter().any(|native| {
                native.id == row.id && same_file(&native.file, &group.file) && native.repeater.is_none()
                    && native.interval == 0.0 && native.range == 0.0 && native.spline_index >= 0
                    && source.tile.splines.get(native.spline_index as usize).is_some_and(|s| s.id == row.spline)
            }));
            if !valid || !owned.insert((row.tile, row.id)) {
                return Err(format!("Objektreihe {}: Anhang {} fehlt oder ist nicht eindeutig", group.id, row.id));
            }
        }
    }
    let mut edits = world.roadside_edits.lock();
    if !edits.initialized { edits.sources = sources; edits.groups = groups; edits.initialized = true; }
    Ok(())
}
fn current_tiles(world: &World) -> HashMap<TileKey, Source> {
    let missing: Vec<_> = world.map_tiles().into_iter().filter(|(_, x, y, _)| !world.roadside_edits.lock().sources.contains_key(&(*x, *y))).collect();
    for (_, x, y, _) in missing {
        if let Ok((tile, editable)) = world.editor_row_source((x, y)) {
            world.roadside_edits.lock().sources.insert((x, y), Source { tile: Arc::new(tile), editable: Arc::new(editable) });
        }
    }
    let mut sources = world.roadside_edits.lock().sources.clone();
    let splines = world.spline_edits.lock();
    let changed: HashSet<_> = splines.changed.keys().chain(splines.added.keys()).map(|(tile, _)| *tile).collect();
    for (key, source) in &mut sources {
        if changed.contains(key) {
            splines.overlay(*key, Arc::make_mut(&mut source.tile));
            Arc::make_mut(&mut source.editable).extend(splines.added.keys().filter(|(tile, _)| tile == key).map(|(_, id)| *id));
        }
    }
    drop(splines);
    let rows = world.roadside_edits.lock();
    for (key, source) in &mut sources {
        if rows.changes.contains_key(key) { rows.overlay(*key, Arc::make_mut(&mut source.tile)); }
    }
    sources
}
fn edges(world: &World, file: &str) -> Option<(f64, f64)> {
    let ty = world.spline_type(file)?;
    let lanes: Vec<_> = ty.def.paths.iter().filter(|p| p.kind == 0).collect();
    if lanes.is_empty() { return None; }
    let lo = lanes.iter().map(|p| p.start[0] as f64 - p.width as f64 * 0.5).fold(f64::INFINITY, f64::min);
    let hi = lanes.iter().map(|p| p.start[0] as f64 + p.width as f64 * 0.5).fold(f64::NEG_INFINITY, f64::max);
    (lo.is_finite() && hi.is_finite() && hi > lo && hi - lo <= 100.0).then_some((lo, hi))
}
fn road_segments(world: &World, tiles: &HashMap<TileKey, Source>) -> HashMap<Key, Segment> {
    let mut bounds = HashMap::new(); let mut result = HashMap::new();
    for (tile, source) in tiles {
        for spline in source.tile.splines.iter().filter(|s| !s.deleted && source.editable.contains(&s.id)) {
            let edge = *bounds.entry(spline.file.clone()).or_insert_with(|| edges(world, &spline.file));
            if let Some(mut edges) = edge {
                if !spline.length.is_finite() || spline.length <= 0.0 { continue; }
                if spline.mirror { edges = (-edges.1, -edges.0); }
                result.insert((*tile, spline.id), Segment { key: (*tile, spline.id), spline: spline.clone(), backwards: false, edges });
            }
        }
    }
    result
}
fn walk(start: Segment, roads: &HashMap<Key, Segment>) -> Result<(Vec<Segment>, bool), String> {
    let mut route = vec![start]; let mut seen = HashSet::new(); seen.insert(route[0].key);
    loop {
        let last = route.last().unwrap(); let c = curve(last.key, &last.spline);
        let link = if last.backwards { last.spline.prev_id } else { last.spline.next_id };
        if link == 0 { break; }
        let end = c.point_at(if last.backwards { 0.0 } else { last.spline.length });
        let mut next = Vec::new();
        for candidate in roads.values().filter(|s| s.key.1 == link) {
            let cc = curve(candidate.key, &candidate.spline);
            for (backwards, neighbour, distance) in [(false, candidate.spline.prev_id, cc.point_at(0.0).distance(end)),
                (true, candidate.spline.next_id, cc.end_point().distance(end))] {
                if neighbour == last.key.1 && distance < 1.0 {
                    let mut s = candidate.clone(); s.backwards = backwards; next.push((distance, s));
                }
            }
        }
        next.sort_by(|a, b| a.0.total_cmp(&b.0));
        if next.len() > 1 && (next[0].0 - next[1].0).abs() < 1e-5 {
            return Err("Mehrdeutiger Straßenanschluss: zuerst die Verbindung prüfen".into());
        }
        let Some((_, next)) = next.into_iter().next() else { break; };
        if !seen.insert(next.key) { return Ok((route, true)); }
        if route.len() >= 500 { return Err("Mehr als 500 Splines: einen kürzeren Bereich wählen".into()); }
        route.push(next);
    }
    Ok((route, false))
}
fn route(start: Key, connected: bool, roads: &HashMap<Key, Segment>) -> Result<Vec<Segment>, String> {
    let first = roads.get(&start).cloned().ok_or("Eine bearbeitbare Straße mit Fahrbahnwegen auswählen, keine Markierung")?;
    if !connected { return Ok(vec![first]); }
    let (forward, circle) = walk(first.clone(), roads)?;
    if circle { return Ok(forward); }
    let mut first = first; first.backwards = true;
    let (mut behind, _) = walk(first, roads)?;
    behind.remove(0); behind.reverse();
    for s in &mut behind { s.backwards = !s.backwards; }
    let mut seen: HashSet<_> = forward.iter().map(|s| s.key).collect();
    behind.retain(|s| seen.insert(s.key)); behind.extend(forward); Ok(behind)
}
struct ProjectionSegment {
    curve: SplineCurve, length: f64, backwards: bool, accumulated: f64,
    samples: Vec<DVec2>, step: f64, bounds: [f64; 4],
}
/// Cache the road samples once; distant map tiles must not make each preview expensive.
struct Projection(Vec<ProjectionSegment>);
impl Projection {
    fn new(route: &[Segment]) -> Self {
        let mut accumulated = 0.0; let mut segments = Vec::new();
        for segment in route {
            let c = curve(segment.key, &segment.spline); let length = segment.spline.length;
            let steps = (length / 5.0).ceil().clamp(1.0, 4096.0) as usize;
            let step = length / steps as f64;
            let samples: Vec<_> = (0..=steps).map(|n| c.point_at(n as f64 * step).truncate()).collect();
            let mut bounds = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
            for p in &samples { bounds[0] = bounds[0].min(p.x); bounds[1] = bounds[1].min(p.y);
                bounds[2] = bounds[2].max(p.x); bounds[3] = bounds[3].max(p.y); }
            // An arc between samples cannot travel farther than its arc length.
            bounds[0] -= step; bounds[1] -= step; bounds[2] += step; bounds[3] += step;
            segments.push(ProjectionSegment { curve: c, length, backwards: segment.backwards, accumulated, samples, step, bounds });
            accumulated += length;
        }
        Self(segments)
    }
    fn project(&self, point: DVec2, maximum: f64) -> Option<(f64, f64, usize, f64)> {
        let mut best: Option<(f64, f64, usize, f64)> = None;
        for (index, segment) in self.0.iter().enumerate() {
            let limit = best.as_ref().map_or(maximum, |(_, old, _, _)| *old);
            let [x0, y0, x1, y1] = segment.bounds;
            let closest = DVec2::new(point.x.clamp(x0, x1), point.y.clamp(y0, y1));
            if closest.distance_squared(point) > limit * limit { continue; }
            let sample = segment.samples.iter().enumerate().min_by(|(_, a), (_, b)| {
                a.distance_squared(point).total_cmp(&b.distance_squared(point))
            }).unwrap().0;
            let c = &segment.curve; let step = segment.step;
            let mut lo = ((sample as f64 - 1.0) * step).max(0.0);
            let mut hi = ((sample as f64 + 1.0) * step).min(segment.length);
            for _ in 0..24 {
                let a = lo + (hi - lo) / 3.0; let b = hi - (hi - lo) / 3.0;
                if c.point_at(a).truncate().distance_squared(point) < c.point_at(b).truncate().distance_squared(point) { hi = b; } else { lo = a; }
            }
            let u = (lo + hi) * 0.5; let distance = c.point_at(u).truncate().distance(point);
            if distance <= limit {
                best = Some((segment.accumulated + if segment.backwards { segment.length - u } else { u }, distance, index, u));
            }
        }
        best
    }
}
fn junction_gaps(route: &[Segment], roads: &HashMap<Key, Segment>, settings: &Settings) -> Vec<[f64; 2]> {
    let mut gaps = settings.manual_gaps.clone(); let gap = settings.junction_gap;
    if gap <= 0.0 { return gaps; }
    let own: HashSet<_> = route.iter().map(|s| s.key).collect();
    let projection = Projection::new(route);
    for other in roads.values().filter(|s| !own.contains(&s.key)) {
        let oc = curve(other.key, &other.spline);
        for u in [0.0, other.spline.length] {
            let point = oc.point_at(u).truncate();
            if let Some((station, distance, segment, local)) = projection.project(point, 101.0) {
                let road = &route[segment];
                let width = (road.edges.1 - road.edges.0 + other.edges.1 - other.edges.0) * 0.5;
                if distance > width + 1.0 { continue; }
                let c = curve(road.key, &road.spline);
                // An overpass at the same map position is not a road mouth.
                if (c.point_at(local).z - oc.point_at(u).z).abs() > 2.5 { continue; }
                let angle = (c.heading_at(local) - oc.heading_at(u)).to_radians();
                // Nearby parallel carriageways are not side-road mouths.
                if angle.cos().abs() < 0.94 { gaps.push([(station - gap).max(0.0), station + gap]); }
            }
        }
    }
    let mut distance = 0.0;
    for segment in route {
        for (station, link) in [(distance, if segment.backwards { segment.spline.next_id } else { segment.spline.prev_id }),
            (distance + segment.spline.length, if segment.backwards { segment.spline.prev_id } else { segment.spline.next_id })] {
            // A crossing object ends the road-spline chain; leave its mouth clear too.
            let c = curve(segment.key, &segment.spline);
            let endpoint = c.point_at(if station == distance { if segment.backwards { segment.spline.length } else { 0.0 } }
                else if segment.backwards { 0.0 } else { segment.spline.length });
            if link != 0 && !roads.values().filter(|s| s.key.1 == link).any(|s| {
                let c = curve(s.key, &s.spline);
                c.point_at(0.0).distance(endpoint) < 1.0 || c.end_point().distance(endpoint) < 1.0
            }) {
                gaps.push([(station - gap).max(0.0), station + gap]);
            }
        }
        distance += segment.spline.length;
    }
    gaps.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut merged: Vec<[f64; 2]> = Vec::new();
    for gap in gaps {
        if let Some(last) = merged.last_mut().filter(|last| gap[0] <= last[1]) { last[1] = last[1].max(gap[1]); }
        else { merged.push(gap); }
    }
    merged
}
pub fn gap_station(world: &World, start: Key, settings: &Settings, point: DVec3) -> Result<f64, String> {
    initialize(world)?; let roads = road_segments(world, &current_tiles(world));
    let route = route(start, settings.connected, &roads)?;
    let (station, distance, index, _) = Projection::new(&route).project(point.truncate(), 54.0).ok_or("Straße nicht gefunden")?;
    if distance > (route[index].edges.1 - route[index].edges.0) * 0.5 + 4.0 {
        return Err("Die Lücke direkt auf der gewählten Straße markieren".into());
    }
    Ok(station)
}
fn existing_positions(world: &World, tiles: &HashMap<TileKey, Source>, file: &str, excluded: &HashSet<i64>) -> Vec<DVec3> {
    let index = world.index(); let mut result = Vec::new(); let size = omsi_map::tile_size();
    for (key, source) in tiles {
        let origin = DVec2::new(key.0 as f64 * size, key.1 as f64 * size);
        for row in source.tile.spline_attachments.iter().filter(|r| !excluded.contains(&r.id) && same_file(&r.file, file)) {
            result.extend(crate::tiles::tile_row_objects(row, &source.tile.splines, origin, Some(&index)).into_iter().map(|(_, p)| p.pose.pos));
        }
        for object in source.tile.objects.iter().filter(|o| same_file(&o.file, file)) {
            result.push(DVec3::new(origin.x + object.pos[0], origin.y + object.pos[1], object.pos[2]));
        }
    }
    result
}
/// Both sides share one station count; reversing an authored segment flips its lateral side.
fn station_points(route: &[Segment], settings: &Settings, file: &str, chain_offsets: &HashMap<Key, f64>) -> Result<Plan, String> {
    settings.validate()?;
    if route.is_empty() || route.iter().any(|s| !s.spline.length.is_finite() || s.spline.length <= 0.0) {
        return Err("Straße hat keine gültige Länge".into());
    }
    let length: f64 = route.iter().map(|s| s.spline.length).sum();
    if !length.is_finite() { return Err("Straße hat keine gültige Länge".into()); }
    let end = if settings.range > 0.0 { (settings.start + settings.range).min(length) } else { length };
    if settings.start > end + 1e-6 { return Err("Startversatz liegt hinter dem Ende der Straße".into()); }
    let count = ((end - settings.start) / settings.interval + 1e-8).floor().max(0.0) as usize + 1;
    let sides = if settings.sides == 3 { 2 } else { 1 };
    if count.saturating_mul(sides) > LIMIT { return Err("Mehr als 4000 Objekte: Abstand erhöhen oder Bereich verkürzen".into()); }
    let mut plan = Plan { length, segments: route.len(), ..Default::default() };
    let (mut segment, mut distance) = (0, 0.0);
    for ordinal in 0..count {
        let station = settings.start + ordinal as f64 * settings.interval;
        // A shared endpoint belongs to exactly one segment.
        while segment + 1 < route.len() && station > distance + route[segment].spline.length + 1e-6 {
            distance += route[segment].spline.length; segment += 1;
        }
        let s = &route[segment]; let c = curve(s.key, &s.spline);
        let along = (station - distance).clamp(0.0, s.spline.length);
        let u = if s.backwards { s.spline.length - along } else { along };
        for side in [1u8, 2] {
            if settings.sides & side == 0 { continue; }
            let right = if s.backwards { side == 1 } else { side == 2 };
            let lateral = if right { s.edges.1 + settings.margin } else { s.edges.0 - settings.margin };
            let pos = c.offset_point(u, lateral, settings.height);
            let rotation = settings.rotation + if side == 2 { 180.0 } else { 0.0 } + if s.backwards { 180.0 } else { 0.0 };
            let row = SplineAttachment { file: file.into(), offset: [lateral, settings.height, chain_offsets.get(&s.key).copied().unwrap_or(0.0) + u],
                rot: [rotation, 0.0, 0.0], interval: 0.0, range: 0.0, tilt: false, ..Default::default() };
            plan.points.push(Point { key: s.key, ordinal, side, pos, row });
        }
    }
    Ok(plan)
}
pub fn plan(world: &World, start: Key, settings: &Settings, file: &str, excluded: &HashSet<i64>) -> Result<Plan, String> {
    initialize(world)?;
    let tiles = current_tiles(world); let roads = road_segments(world, &tiles);
    let route = route(start, settings.connected, &roads)?;
    let index = world.index();
    let offsets = route.iter().map(|s| (s.key, crate::tiles::attachment_chain_offset(&index, &s.spline))).collect();
    let mut plan = station_points(&route, settings, file, &offsets)?;
    plan.gaps = junction_gaps(&route, &roads, settings);
    let mut positions: HashMap<(i64, i64), Vec<DVec2>> = HashMap::new();
    for point in existing_positions(world, &tiles, file, excluded) {
        positions.entry((point.x.floor() as i64, point.y.floor() as i64)).or_default().push(point.truncate());
    }
    let gaps = &plan.gaps;
    let before = plan.points.len();
    plan.points.retain(|p| {
        let station = settings.start + p.ordinal as f64 * settings.interval;
        !gaps.iter().any(|[a, b]| station >= *a - 1e-6 && station <= *b + 1e-6)
            && !(-1..=1).any(|dx| (-1..=1).any(|dy| {
                positions.get(&(p.pos.x.floor() as i64 + dx, p.pos.y.floor() as i64 + dy))
                    .is_some_and(|points| points.iter().any(|q| p.pos.truncate().distance_squared(*q) < 0.75 * 0.75))
            }))
    });
    plan.skipped = before - plan.points.len();
    if settings.ground {
        for point in &mut plan.points {
            let ground = world.editor_terrain_height(point.pos.x, point.pos.y).ok_or("Gelände für die Pfostenhöhe nicht verfügbar")?;
            let new_height = ground + settings.height;
            point.row.offset[1] += new_height - point.pos.z; point.pos.z = new_height;
        }
    }
    Ok(plan)
}
fn mark(edits: &mut Edits, tile: TileKey) { edits.dirty_tiles.insert(tile); edits.save_tiles.insert(tile); }
fn install_plan(world: &World, group: &mut Group, plan: Plan) -> Result<(), String> {
    let mut rows = Vec::new(); let mut replacements = Vec::new();
    let existing: HashMap<_, _> = group.rows.iter().map(|r| ((r.ordinal, r.side), r.id)).collect();
    for mut point in plan.points {
        let id = existing.get(&(point.ordinal, point.side)).copied()
            .or_else(|| world.allocate_editor_id()).ok_or("Keine freie Objektkennung")?;
        point.row.id = id;
        rows.push(RowId { id, tile: point.key.0, spline: point.key.1, ordinal: point.ordinal, side: point.side });
        replacements.push((point.key.0, id, RowEdit { spline: point.key.1, row: point.row }));
    }
    let mut edits = world.roadside_edits.lock();
    let mut changed = group.rows != rows;
    let retained: HashSet<_> = rows.iter().map(|r| (r.tile, r.id)).collect();
    for old in &group.rows {
        if !retained.contains(&(old.tile, old.id)) {
            edits.changes.entry(old.tile).or_default().insert(old.id, None); mark(&mut edits, old.tile); changed = true;
        }
    }
    for (tile, id, edit) in replacements {
        let same = edits.changes.get(&tile).and_then(|rows| rows.get(&id)).and_then(|row| row.as_ref()) == Some(&edit);
        if !same { edits.changes.entry(tile).or_default().insert(id, Some(edit)); mark(&mut edits, tile); changed = true; }
    }
    if changed { mark(&mut edits, group.start.0); }
    group.rows = rows; Ok(())
}
pub fn apply(world: &World, start: Key, settings: Settings, file: String) -> Result<usize, String> {
    settings.validate()?; world.editor_object_type(&file)?;
    initialize(world)?;
    let existing = world.roadside_edits.lock().groups.iter().rev()
        .find(|g| g.start == start && same_file(&g.file, &file)).cloned();
    let excluded = existing.as_ref().map(|g| g.rows.iter().map(|r| r.id).collect()).unwrap_or_default();
    let preview = plan(world, start, &settings, &file, &excluded)?;
    if preview.points.is_empty() && existing.is_none() { return Err("Keine freien Positionen: Lücken, Startversatz und vorhandene Objekte prüfen".into()); }
    let count = preview.points.len();
    let mut group = match existing { Some(mut group) => { group.settings = settings; group.file = file; group },
        None => Group { id: world.allocate_editor_id().ok_or("Keine freie Reihenkennung")?, start, file, settings, rows: Vec::new() } };
    let id = group.id;
    let before = world.roadside_edits.lock().groups.clone();
    install_plan(world, &mut group, preview)?;
    let mut edits = world.roadside_edits.lock(); edits.undo.push(before);
    if let Some(old) = edits.groups.iter_mut().find(|g| g.id == id) { *old = group; } else { edits.groups.push(group); }
    // Changed settings alone also need a rewritten recipe, even when all native positions agree.
    mark(&mut edits, start.0);
    log::info!("Objektreihe {id}: {count} Objekte eingesetzt"); Ok(count)
}
/// Rebuild the native records after road edits; never reset unrelated scenery objects.
pub fn refresh(world: &World) -> Result<(), String> {
    if !world.roadside_edits.lock().initialized { return Ok(()); }
    let mut groups = world.roadside_edits.lock().groups.clone();
    let mut plans = Vec::new();
    for group in &groups {
        let excluded = group.rows.iter().map(|r| r.id).collect();
        let next = match plan(world, group.start, &group.settings, &group.file, &excluded) {
            Ok(plan) => plan,
            Err(_) if deleted_road(world, group.start) => Plan::default(),
            Err(error) => return Err(error),
        };
        plans.push(next);
    }
    // Validate every recipe before changing any native record.
    let rollback = world.roadside_edits.lock().clone();
    for (group, next) in groups.iter_mut().zip(plans) {
        if let Err(error) = install_plan(world, group, next) {
            *world.roadside_edits.lock() = rollback; return Err(error);
        }
    }
    world.roadside_edits.lock().groups = groups; Ok(())
}
fn deleted_road(world: &World, key: Key) -> bool {
    let changed = world.spline_edits.lock().current(key);
    if let Some(spline) = changed { return spline.deleted; }
    world.roadside_edits.lock().sources.get(&key.0)
        .is_some_and(|source| source.tile.splines.iter().any(|s| s.id == key.1 && s.deleted))
}
pub fn undo(world: &World) -> Result<(), String> {
    let rollback = world.roadside_edits.lock().clone();
    let (previous, current) = {
        let mut edits = world.roadside_edits.lock();
        (edits.undo.pop().ok_or("Keine neue Objektreihe zum Rückgängigmachen")?, edits.groups.clone())
    };
    {
        let mut edits = world.roadside_edits.lock();
        for group in current {
            for row in group.rows { edits.changes.entry(row.tile).or_default().insert(row.id, None); mark(&mut edits, row.tile); }
            mark(&mut edits, group.start.0);
        }
        edits.groups = previous;
    }
    if let Err(error) = refresh(world) { *world.roadside_edits.lock() = rollback; return Err(error); }
    Ok(())
}
pub fn remove(world: &World, selected: Key) -> Result<(), String> {
    initialize(world)?;
    let mut edits = world.roadside_edits.lock();
    let group = edits.groups.iter().rposition(|g| g.start == selected || g.rows.iter().any(|r| r.tile == selected.0 && r.spline == selected.1)).ok_or("Auf dieser Straße wurde noch keine eigene Objektreihe angelegt")?;
    let previous = edits.groups.clone(); edits.undo.push(previous);
    let group = edits.groups.remove(group);
    for row in group.rows { edits.changes.entry(row.tile).or_default().insert(row.id, None); mark(&mut edits, row.tile); }
    mark(&mut edits, group.start.0); Ok(())
}
fn read_groups(text: &str) -> Result<Vec<Group>, String> {
    let lines: Vec<_> = text.lines().collect(); let mut groups = Vec::new(); let mut i = 0;
    while i < lines.len() {
        let tag = lines[i].trim().to_ascii_lowercase();
        if tag == "[splineattachement]" || tag == "[splineattachement_repeater]" {
            let shift = if tag.ends_with("_repeater]") { 2 } else { 0 };
            let count = lines.get(i + 14 + shift).and_then(|s| s.trim().parse::<usize>().ok()).ok_or("Objektbeschriftungen fehlen")?;
            i = (i + 15 + shift).checked_add(count).ok_or("Zu viele Objektbeschriftungen")?;
            if i > lines.len() { return Err("Unvollständiger Objekt-Anhang".into()); }
            continue;
        }
        if tag == META {
            let data = lines.get(i + 1).ok_or("Unvollständige Objektreihe")?;
            groups.push(serde_json::from_str(data).map_err(|e| format!("Objektreihe nicht lesbar: {e}"))?);
            i += 2; continue;
        }
        i += 1;
    }
    Ok(groups)
}
/// Preserve all native records and attachment indices; only touch our own row IDs.
pub fn rewrite(text: &str, key: TileKey, edits: &Edits) -> Result<(String, usize), String> {
    if !edits.save_tiles.contains(&key) { return Ok((text.into(), 0)); }
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let changes = edits.changes.get(&key); let mut out = String::new(); let mut i = 0;
    let mut splines = Vec::new();
    while i < lines.len() {
        let tag = lines[i].trim().to_ascii_lowercase();
        if tag == "[spline]" || tag == "[spline_h]" {
            let id = lines.get(i + 3).and_then(|s| s.trim().parse::<i64>().ok()).ok_or("Spline-Kennung fehlt")?;
            splines.push(id);
        }
        if tag == META {
            if i + 1 >= lines.len() { return Err("Unvollständige Objektreihe".into()); }
            if out.ends_with(&format!("{newline}{newline}")) { out.truncate(out.len() - newline.len()); }
            i += 2; continue;
        }
        if tag == "[splineattachement]" || tag == "[splineattachement_repeater]" {
            let shift = if tag.ends_with("_repeater]") { 2 } else { 0 };
            let id = lines.get(i + 3 + shift).and_then(|s| s.trim().parse::<i64>().ok()).ok_or("Objektkennung fehlt")?;
            let count_line = i + 14 + shift;
            let count = lines.get(count_line).and_then(|s| s.trim().parse::<usize>().ok()).ok_or("Objektbeschriftungen fehlen")?;
            let end = count_line.checked_add(1).and_then(|n| n.checked_add(count)).ok_or("Zu viele Objektbeschriftungen")?;
            if end > lines.len() { return Err("Unvollständiger Objekt-Anhang".into()); }
            if changes.is_some_and(|c| c.contains_key(&id)) {
                if out.ends_with(&format!("{newline}{newline}")) { out.truncate(out.len() - newline.len()); }
            } else { for line in &lines[i..end] { out.push_str(line); } }
            // Labels may themselves look like keywords. They belong to this record.
            i = end; continue;
        }
        out.push_str(lines[i]); i += 1;
    }
    if !out.ends_with('\n') { out.push_str(newline); }
    if let Some(changes) = changes {
        let mut rows: Vec<_> = changes.values().flatten().collect(); rows.sort_by_key(|e| e.row.id);
        for edit in rows {
            let index = splines.iter().position(|id| *id == edit.spline).ok_or("Straße der Objektreihe fehlt beim Speichern")?;
            let row = &edit.row;
            let fields = vec!["[splineAttachement]".to_string(), "0".into(), row.file.clone(), row.id.to_string(), index.to_string(),
                format!("{:.12}", row.offset[0]), format!("{:.12}", row.offset[1]), format!("{:.12}", row.offset[2]),
                format!("{:.12}", row.rot[0]), "0".into(), "0".into(), "0".into(), "0".into(), "0".into(), row.strings.len().to_string()];
            out.push_str(newline); out.push_str(&fields.join(newline)); out.push_str(newline);
            for label in &row.strings { out.push_str(label); out.push_str(newline); }
        }
    }
    for group in edits.groups.iter().filter(|g| g.start.0 == key) {
        out.push_str(newline); out.push_str(META); out.push_str(newline);
        out.push_str(&serde_json::to_string(group).map_err(|e| e.to_string())?); out.push_str(newline);
    }
    Ok((out, 1))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field { Interval, Margin, Start, Range, Height, Rotation, JunctionGap }
impl Field {
    pub fn title(self) -> &'static str { match self { Self::Interval => "Abstand (m)", Self::Margin => "Abstand zum Rand (m)",
        Self::Start => "Startversatz (m)", Self::Range => "Bereich (m; 0 = ganz)", Self::Height => "Höhenversatz (m)", Self::Rotation => "Drehung (Grad)", Self::JunctionGap => "Abstand zur Einmündung (m)" } }
}
#[derive(Clone, Copy)]
pub enum Command { Close, Catalog, Edit(Field), Adjust(Field, f64), Sides(u8), Connected(bool), Ground(bool), Preview, Apply, Undo, Remove, Save, PickGap, ClearGaps }
pub struct Input { pub field: Field, pub text: String, pub replace: bool }
pub struct Window {
    pub start: Option<Key>, pub file: String, pub settings: Settings, pub input: Option<Input>,
    pub message: String, pub error: Option<String>, pub preview: Plan,
    pub rects: Vec<([f32; 4], Command)>, pub rect: Option<[f32; 4]>, pub can_undo: bool, pub gap_pick: Option<Option<f64>>,
}
impl Window {
    pub fn new(start: Option<Key>) -> Self {
        Self { start, file: DEFAULT_OBJECT.into(), settings: Settings::default(), input: None,
            message: "Straße anklicken · Blau = geplante Objekte · Einsetzen übernimmt die Vorschau".into(), error: None,
            preview: Plan::default(), rects: Vec::new(), rect: None, can_undo: false, gap_pick: None }
    }
    pub fn hit(&self, p: (f32, f32)) -> Option<Command> { self.rects.iter().rev().find(|(r, _)| p.0 >= r[0] && p.0 <= r[2] && p.1 >= r[1] && p.1 <= r[3]).map(|(_, c)| *c) }
    pub fn contains(&self, p: (f32, f32)) -> bool { self.rect.is_some_and(|r| p.0 >= r[0] && p.0 <= r[2] && p.1 >= r[1] && p.1 <= r[3]) }
    pub fn value(&self, f: Field) -> f64 { match f { Field::Interval => self.settings.interval, Field::Margin => self.settings.margin,
        Field::Start => self.settings.start, Field::Range => self.settings.range, Field::Height => self.settings.height, Field::Rotation => self.settings.rotation, Field::JunctionGap => self.settings.junction_gap } }
    pub fn edit(&mut self, field: Field) { self.input = Some(Input { field, text: format!("{:.2}", self.value(field)), replace: true }); }
    pub fn set(&mut self, field: Field, value: f64) -> Result<(), String> {
        let mut settings = self.settings.clone();
        match field { Field::Interval => settings.interval = value, Field::Margin => settings.margin = value,
            Field::Start => settings.start = value, Field::Range => settings.range = value, Field::Height => settings.height = value, Field::Rotation => settings.rotation = value, Field::JunctionGap => settings.junction_gap = value }
        settings.validate()?; self.settings = settings; Ok(())
    }
    pub fn commit(&mut self) -> bool {
        let Some(input) = self.input.take() else { return true; };
        let value = input.text.trim().replace(',', ".").parse::<f64>().map_err(|_| "Eine Zahl eingeben".to_string());
        match value.and_then(|value| self.set(input.field, value)) { Ok(()) => true,
            Err(error) => { self.message = error; self.input = Some(input); false } }
    }
    pub fn select(&mut self, world: &World, start: Option<Key>) {
        self.start = start; self.settings.manual_gaps.clear(); self.gap_pick = None;
        if initialize(world).is_ok() {
            if let Some(group) = world.roadside_edits.lock().groups.iter().rev().find(|g| Some(g.start) == start
                || g.rows.iter().any(|r| Some((r.tile, r.spline)) == start)).cloned() {
                self.start = Some(group.start); self.file = group.file; self.settings = group.settings;
                self.message = "Gespeicherte Reihe gewählt · Einstellungen ändern und Vorschau einsetzen".into();
            }
        }
        self.refresh(world);
    }
    pub fn refresh(&mut self, world: &World) {
        self.can_undo = world.roadside_edits.lock().can_undo();
        let excluded = world.roadside_edits.lock().groups.iter().rev()
            .find(|g| Some(g.start) == self.start && same_file(&g.file, &self.file))
            .map(|g| g.rows.iter().map(|r| r.id).collect()).unwrap_or_default();
        let result = world.editor_object_type(&self.file).and_then(|_| self.start.ok_or_else(|| "Zuerst eine Straße anklicken".to_string())
            .and_then(|start| plan(world, start, &self.settings, &self.file, &excluded)));
        match result { Ok(plan) => { self.preview = plan; self.error = None; },
            Err(error) => { self.preview = Plan::default(); self.error = Some(error); } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn segment(key: Key, y: f64, length: f64) -> Segment {
        Segment { key, backwards: false, edges: (-3.5, 3.5), spline: MapSpline { id: key.1,
            file: "Splines/synthetic-row.sli".into(), pos: [100.0, y, 0.0], length,
            map_chain_offset: Some(0.0), ..Default::default() } }
    }
    fn parsed(text: &str) -> Tile { Tile::parse(&omsi_cfg::CfgFile::from_str("row.map", text)) }
    fn spline_record(s: &MapSpline) -> String {
        format!("[spline]\n0\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n0\n0\n0\n0\n0\n0\n{}\n",
            s.file, s.id, s.prev_id, s.next_id, s.pos[0], s.pos[2], s.pos[1], s.heading,
            s.length, s.radius, s.map_chain_offset.unwrap_or(0.0))
    }
    fn native_row(id: i64, index: i32, labels: &[&str], repeater: bool) -> String {
        let header = if repeater { "[splineAttachement_repeater]\n0\n0\n2" } else { "[splineAttachement]\n0" };
        format!("{header}\nunrelated.sco\n{id}\n{index}\n4\n0\n20\n180\n0\n0\n50\n200\n0\n{}\n{}",
            labels.len(), labels.iter().map(|s| format!("{s}\n")).collect::<String>())
    }

    #[test]
    fn distance_raster_crosses_tiles_and_native_chain_offsets_without_a_restart() {
        let size = omsi_map::tile_size();
        let mut a = segment(((0, 0), 1), size - 120.0, 120.0); a.spline.next_id = 2; a.spline.map_chain_offset = Some(600.0);
        let mut b = segment(((0, 1), 2), 0.0, 80.0); b.spline.prev_id = 1; b.spline.map_chain_offset = Some(720.0);
        let route = vec![a, b]; let offsets = HashMap::from([(route[0].key, 600.0), (route[1].key, 720.0)]);
        let plan = station_points(&route, &Settings::default(), DEFAULT_OBJECT, &offsets).unwrap();
        assert_eq!(plan.points.len(), 10);
        for point in &plan.points {
            assert!((point.pos.y - (size - 120.0 + point.ordinal as f64 * 50.0)).abs() < 1e-8);
            let s = route.iter().find(|s| s.key == point.key).unwrap();
            let origin = DVec2::new(s.key.0.0 as f64 * size, s.key.0.1 as f64 * size);
            let native = crate::tiles::tile_row_objects(&point.row, std::slice::from_ref(&s.spline), origin, None);
            assert_eq!(native.len(), 1); assert!(native[0].1.pose.pos.distance(point.pos) < 1e-8);
        }
        let after = plan.points.iter().find(|p| p.ordinal == 3 && p.side == 1).unwrap();
        assert_eq!(after.key, ((0, 1), 2)); assert!((after.row.offset[2] - 750.0).abs() < 1e-8);
    }
    #[test]
    fn reversed_segment_keeps_both_sides_and_shared_endpoints_unique() {
        let mut a = segment(((0, 0), 1), 0.0, 100.0); a.spline.next_id = 2;
        let mut b = segment(((0, 0), 2), 200.0, 100.0); b.spline.heading = 180.0;
        b.spline.next_id = 1; b.backwards = true;
        let route = vec![a, b];
        let plan = station_points(&route, &Settings::default(), DEFAULT_OBJECT, &HashMap::new()).unwrap();
        assert_eq!(plan.points.iter().filter(|p| (p.pos.y - 100.0).abs() < 1e-6).count(), 2);
        for point in &plan.points {
            assert!((point.pos.y - point.ordinal as f64 * 50.0).abs() < 1e-6);
            assert!((point.pos.x - if point.side == 1 { 96.0 } else { 104.0 }).abs() < 1e-6);
        }
        let after = plan.points.iter().find(|p| p.ordinal == 3 && p.side == 1).unwrap();
        assert_eq!(after.row.offset[0], 4.0); assert_eq!(after.row.rot[0], 180.0);
    }
    #[test]
    fn curved_rows_use_arc_distance_and_do_not_straighten_the_road() {
        let mut s = segment(((0, 0), 1), 0.0, 200.0); s.spline.radius = 100.0;
        let c = curve(s.key, &s.spline);
        let plan = station_points(&[s], &Settings::default(), DEFAULT_OBJECT, &HashMap::new()).unwrap();
        for p in &plan.points {
            let u = p.ordinal as f64 * 50.0;
            assert!(p.pos.distance(c.offset_point(u, if p.side == 1 { -4.0 } else { 4.0 }, 0.0)) < 1e-8);
        }
    }
    #[test]
    fn connected_walk_resolves_reused_ids_by_position_and_stops_at_a_crossing() {
        let mut a = segment(((0, 0), 1), 0.0, 100.0); a.spline.next_id = 2;
        let mut b = segment(((0, 0), 2), 100.0, 100.0); b.spline.prev_id = 1; b.spline.next_id = 99;
        let mut unrelated = b.clone(); unrelated.key = ((20, 20), 2);
        let roads = HashMap::from([(a.key, a.clone()), (b.key, b.clone()), (unrelated.key, unrelated)]);
        let route = route(a.key, true, &roads).unwrap();
        assert_eq!(route.iter().map(|s| s.key).collect::<Vec<_>>(), vec![a.key, b.key]);
        let gaps = junction_gaps(&route, &roads, &Settings::default());
        assert!(gaps.iter().any(|g| (g[0] - 192.0).abs() < 1e-8 && (g[1] - 208.0).abs() < 1e-8));
    }
    #[test]
    fn a_closed_road_chain_does_not_walk_forever() {
        let mut a = segment(((0, 0), 1), 0.0, 100.0); a.spline.next_id = 2; a.spline.prev_id = 2;
        let mut b = segment(((0, 0), 2), 100.0, 100.0); b.spline.heading = 180.0; b.spline.prev_id = 1; b.spline.next_id = 1;
        let roads = HashMap::from([(a.key, a.clone()), (b.key, b)]);
        assert_eq!(route(a.key, true, &roads).unwrap().len(), 2);
    }
    #[test]
    fn side_roads_leave_a_gap_but_parallel_roads_and_overpasses_do_not() {
        let road = segment(((0, 0), 1), 0.0, 200.0);
        let mut side = segment(((0, 0), 2), 50.0, 40.0); side.spline.pos[0] = 104.0; side.spline.heading = 90.0;
        let mut parallel = segment(((0, 0), 3), 100.0, 40.0); parallel.spline.pos[0] = 104.0;
        let mut bridge = side.clone(); bridge.key.1 = 4; bridge.spline.id = 4; bridge.spline.pos = [104.0, 150.0, 6.0];
        let roads = HashMap::from([(road.key, road.clone()), (side.key, side), (parallel.key, parallel), (bridge.key, bridge)]);
        let settings = Settings { manual_gaps: vec![[20.0, 30.0]], ..Default::default() };
        let gaps = junction_gaps(&[road], &roads, &settings);
        assert_eq!(gaps.len(), 2); assert_eq!(gaps[0], [20.0, 30.0]);
        assert!((gaps[1][0] - 42.0).abs() < 0.001); assert!((gaps[1][1] - 58.0).abs() < 0.001);
    }
    #[test]
    fn numeric_limits_reject_invalid_lengths_nan_and_unbounded_counts() {
        let s = segment(((0, 0), 1), 0.0, 5000.0);
        let dense = Settings { interval: 1.0, ..Default::default() };
        assert!(station_points(std::slice::from_ref(&s), &dense, DEFAULT_OBJECT, &HashMap::new()).is_err());
        let nan = Settings { margin: f64::NAN, ..Default::default() }; assert!(nan.validate().is_err());
        assert!(station_points(&[], &Settings::default(), DEFAULT_OBJECT, &HashMap::new()).is_err());
        let mut broken = s; broken.spline.length = f64::NAN;
        assert!(station_points(&[broken], &Settings::default(), DEFAULT_OBJECT, &HashMap::new()).is_err());
    }
    #[test]
    fn native_save_preserves_foreign_rows_labels_repeater_indices_and_recipe() {
        let a = segment(((0, 0), 1), 0.0, 100.0); let b = segment(((0, 0), 2), 100.0, 100.0);
        let foreign = native_row(70, 1, &["unrelated label", "[not_a_keyword]", META, "not JSON"], true);
        let own = native_row(80, 0, &["old label"], false);
        let text = format!("[version]\n14\n{}{}[terrain]\n\n{foreign}\n{own}", spline_record(&a.spline), spline_record(&b.spline)).replace('\n', "\r\n");
        let mut edits = Edits::default(); edits.save_tiles.insert((0, 0));
        let row = SplineAttachment { id: 80, file: DEFAULT_OBJECT.into(), offset: [-4.0, 0.25, 50.0], strings: vec!["new label".into()], ..Default::default() };
        edits.changes.insert((0, 0), HashMap::from([(80, Some(RowEdit { spline: 2, row }))]));
        edits.groups.push(Group { id: 90, start: a.key, file: DEFAULT_OBJECT.into(), settings: Settings::default(),
            rows: vec![RowId { id: 80, tile: (0, 0), spline: 2, ordinal: 1, side: 1 }] });
        let (saved, _) = rewrite(&text, (0, 0), &edits).unwrap();
        assert!(saved.contains(&foreign.replace('\n', "\r\n")));
        let native = parsed(&saved); assert_eq!(native.splines.len(), 2); assert_eq!(native.spline_attachments.len(), 2);
        let changed = native.spline_attachments.iter().find(|r| r.id == 80).unwrap();
        assert_eq!(changed.spline_index, 1); assert_eq!(changed.strings, vec!["new label"]);
        assert_eq!(read_groups(&saved).unwrap()[0].rows[0].id, 80);
        assert_eq!(rewrite(&saved, (0, 0), &edits).unwrap().0, saved);
        edits.changes.get_mut(&(0, 0)).unwrap().insert(80, None); edits.groups.clear();
        let removed = rewrite(&saved, (0, 0), &edits).unwrap().0;
        assert_eq!(parsed(&removed).spline_attachments.len(), 1); assert!(read_groups(&removed).unwrap().is_empty());
    }
    #[test]
    fn overlay_remaps_indices_and_omits_rows_of_deleted_roads() {
        let a = segment(((0, 0), 1), 0.0, 100.0); let mut b = segment(((0, 0), 2), 0.0, 100.0); b.spline.deleted = true;
        let mut tile = Tile { splines: vec![a.spline, b.spline], spline_attachments: vec![SplineAttachment { id: 70, ..Default::default() }], ..Default::default() };
        let mut edits = Edits::default(); edits.changes.insert((0, 0), HashMap::from([
            (80, Some(RowEdit { spline: 1, row: SplineAttachment { id: 80, spline_index: 50, ..Default::default() } })),
            (81, Some(RowEdit { spline: 2, row: SplineAttachment { id: 81, ..Default::default() } }))]));
        edits.overlay((0, 0), &mut tile);
        assert_eq!(tile.spline_attachments.iter().map(|r| r.id).collect::<Vec<_>>(), vec![70, 80]);
        assert_eq!(tile.spline_attachments[1].spline_index, 0);
    }
    #[test]
    fn row_update_save_restart_delete_and_undo_keep_ids_and_original_map() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("omsi-object-row-{stamp}"));
        let original = dir.join("original"); let map = original.join("maps/Test"); let content = dir.join("mod");
        std::fs::create_dir_all(&map).unwrap(); std::fs::create_dir_all(original.join("Splines")).unwrap();
        std::fs::create_dir_all(original.join("Sceneryobjects/ADDON_gcW")).unwrap();
        std::fs::write(original.join("Splines/synthetic-row.sli"), "[path]\n0\n0\n0\n7\n2\n").unwrap();
        std::fs::write(original.join("Sceneryobjects/ADDON_gcW/leitpfosten.sco"), "[friendlyname]\nSynthetic post\n").unwrap();
        std::fs::write(map.join("global.cfg"), "[map]\n0\n0\ntile_0_0.map\n").unwrap();
        let road = segment(((0, 0), 1), 20.0, 100.0);
        let original_text = format!("[version]\n14\n{}{}", spline_record(&road.spline), native_row(70, 0, &[], false));
        std::fs::write(map.join("tile_0_0.map"), &original_text).unwrap();
        let world = World::open(&original, &map.join("global.cfg"), 20000101).unwrap();
        assert_eq!(apply(&world, road.key, Settings::default(), DEFAULT_OBJECT.into()).unwrap(), 6);
        let ids: Vec<_> = world.roadside_edits.lock().groups[0].rows.iter().map(|r| r.id).collect();
        assert_eq!(apply(&world, road.key, Settings::default(), DEFAULT_OBJECT.into()).unwrap(), 6);
        assert_eq!(world.roadside_edits.lock().groups.len(), 1);
        assert_eq!(world.roadside_edits.lock().groups[0].rows.iter().map(|r| r.id).collect::<Vec<_>>(), ids);
        let editor = crate::editor::Editor::default(); editor.save(&world, "maps/Test/global.cfg", &content, &original).unwrap();
        assert_eq!(std::fs::read_to_string(map.join("tile_0_0.map")).unwrap(), original_text);
        let saved = content.join("maps/Test/tile_0_0.map"); let saved_text = std::fs::read_to_string(&saved).unwrap();
        assert_eq!(parsed(&saved_text).spline_attachments.len(), 7);
        std::fs::write(content.join("maps/Test/global.cfg"), "[map]\n0\n0\ntile_0_0.map\n").unwrap();
        let reopened = World::open(&original, &content.join("maps/Test/global.cfg"), 20000101).unwrap(); initialize(&reopened).unwrap();
        assert_eq!(reopened.roadside_edits.lock().groups[0].rows.iter().map(|r| r.id).collect::<Vec<_>>(), ids);
        let mut moved = road.spline.clone(); moved.pos[0] += 20.0;
        reopened.spline_edits.lock().changed.insert(road.key, moved); refresh(&reopened).unwrap();
        let tiles = current_tiles(&reopened); let source = &tiles[&(0, 0)].tile;
        let post = source.spline_attachments.iter().find(|r| r.id == ids[0]).unwrap();
        let placed = crate::tiles::tile_row_objects(post, &source.splines, DVec2::ZERO, None);
        assert!((placed[0].1.pose.pos.x - 116.0).abs() < 1e-8);
        remove(&reopened, road.key).unwrap(); assert!(reopened.roadside_edits.lock().groups.is_empty());
        undo(&reopened).unwrap(); assert_eq!(reopened.roadside_edits.lock().groups[0].rows.iter().map(|r| r.id).collect::<Vec<_>>(), ids);
        remove(&reopened, road.key).unwrap();
        let editor = crate::editor::Editor::default(); editor.save(&reopened, "maps/Test/global.cfg", &content, &original).unwrap();
        assert_eq!(parsed(&std::fs::read_to_string(&saved).unwrap()).spline_attachments.len(), 1);
        assert_eq!(std::fs::read_to_string(map.join("tile_0_0.map")).unwrap(), original_text);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// Shared reciprocal road traversal; object junctions and unresolved branches end the route.
pub fn sidewalk_route(world:&World,start:Key,connected:bool)->Result<Vec<(Key,MapSpline,bool)>,String> {
    let tiles=current_tiles(world);let roads=road_segments(world,&tiles);
    Ok(route(start,connected,&roads)?.into_iter().map(|s|(s.key,s.spline,s.backwards)).collect())
}
