//! The object editor: a small part of what OMSI's map editor does, inside the game. The
//! scenery objects a tile places itself (its `[object]` records) can be picked, moved,
//! turned and deleted where they stand, and the tiles changed are written as copies into
//! the content folder - which the game reads before the installation - never into the
//! original map. C captures a selected object and starts click placement; Ctrl+C/V
//! copy and paste through the editor clipboard. Repeated placement uses independent
//! `[object]` records with their labels and tree dimensions. V changes the object type. The ground is shaped with a brush where the view points (raised,
//! lowered, flattened); a tile's ground is written as its `.map.terrain` copy. Splines are
//! edited in its T mode (the timetable is the launcher's Timetable page).
//!
//! Keys while it is on (Ctrl+Shift+E, or the game menu):
//! Enter picks the object nearest the middle of the view, Tab the next nearest;
//! I / K / J / L move it forward, back, left and right as the camera faces, U / O down and
//! up, N / M turn it (half a metre and five degrees a press, a tenth with Shift);
//! Delete takes it away (again: back), Backspace undoes all its edits, C copies it (the
//! copy is then the one edited), V gives a copy the next object of its folder, Ctrl+S
//! saves the changed tiles and Escape leaves the editor. Page Up / Page Down raise and
//! lower the ground under the middle of the view (a quarter metre, a twentieth with
//! Shift), F flattens it to the height at the middle, [ and ] make the brush smaller and
//! larger.

use crate::scene::{ObjectEdit, World};
use glam::{DVec3, Mat4, Vec3};
use hashbrown::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A new object: a copy of `template` (a map object), moved and turned from it, perhaps of
/// another type of the template's folder.
pub struct Added {
    pub template: i64,
    pub tile: (i32, i32),
    pub id: i64,
    /// Its `.sco` (the template's, or one of its folder: `V`).
    pub sco: PathBuf,
    pub base: DVec3,
    pub base_heading: f64,
    pub moved: DVec3,
    pub turned: f64,
    pub deleted: bool,
    /// A directly chosen .sco, without a record to use as a copying template.
    pub standalone: bool,
    pub strings: Vec<String>,
    pub tilt: [f64;2],
    pub shape: Mat4,
    pub ground_height:f64,
    // Billboard helpers share textures rather than TypeGpu, so retain their CPU type here.
    _object_type:Option<std::sync::Arc<crate::scene::ObjectType>>,
    gpu: Option<crate::scene::TileGpu>,
}

#[derive(Clone)]
pub struct ObjectStamp {pub sco:PathBuf,pub heading:f64,pub tilt:[f64;2],pub strings:Vec<String>,pub height_offset:f64}
impl ObjectStamp {
    pub fn new(sco:PathBuf)->Self {Self {sco,heading:0.0,tilt:[0.0;2],strings:Vec::new(),height_offset:0.0}}
}

#[derive(Clone)]
struct JunctionUndo {id:i64,path:PathBuf,before:Vec<(crate::spline_editor::Key,omsi_map::MapSpline)>,after:Vec<(crate::spline_editor::Key,omsi_map::MapSpline)>,at:DVec3,heading:f64}

#[derive(Default)]
pub struct Editor {
    pub audit:Option<crate::audit_events::Audit>,
    pub catalog: Option<crate::asset_catalog::Catalog>,
    pub tile_window: Option<crate::tile_editor::Window>,
    pub text_window: Option<crate::object_text::Window>,
    pub junction_window: Option<crate::junction_builder::Window>,
    junction_history:Vec<JunctionUndo>,
    tilt_baselines:HashMap<i64,[f64;2]>,
    pub sidewalk_window: Option<crate::sidewalk::Window>,
    pub roadside_window: Option<crate::roadside_objects::Window>,
    pub texture_target: Option<TextureTarget>,
    label_history: HashMap<i64, Vec<Vec<String>>>,
    label_resources: HashMap<i64, crate::scene::TileGpu>,
    pub terrain: crate::terrain_editor::TerrainEditor,
    pub placing_asset: Option<crate::asset_catalog::Asset>,
    pub clipboard: Option<ObjectStamp>,
    pub object_stamp: Option<ObjectStamp>,
    pub last_object: Option<ObjectStamp>,
    pub repeat_objects: bool,
    pub align_object: bool,
    pub spline_mode: bool,
    pub splines: crate::spline_editor::SplineEditor,
    pub selected: Option<i64>,
    /// The copies made in this session, and the one being edited (it takes the keys).
    pub added: Vec<Added>,
    pub editing_added: Option<usize>,
    /// The tile of every object edited, by map id (its tile may be unloaded when saving).
    tiles: HashMap<i64, (i32, i32)>,
    /// The map source before this session first saved a tile. The VFS reads our saved
    /// overlay on the next save; applying relative moves again would accumulate them.
    saved_tile_sources: std::sync::Mutex<HashMap<(i32, i32), Vec<u8>>>,
    /// The candidates of the last pick, nearest first (Tab walks them).
    candidates: Vec<PickCandidate>,
    next: usize,
    /// Selected point in model space; the marker follows subsequent movement/rotation.
    pick_anchor: Vec3,
    /// Preserve where the object was grabbed instead of snapping its pivot to the cursor.
    drag_offset: Option<DVec3>,
    tree_trace: HashMap<((i32, i32), i64), TreeTracePose>,
    tree_trace_at: Option<std::time::Instant>,
    tree_trace_log_at: Option<std::time::Instant>,
}

struct TreeTracePose {
    slot: usize,
    pos: DVec3,
    xf: Mat4,
    materials: Vec<usize>,
    uv: Vec<[f32; 2]>,
}

fn same_tree_transform(pos: DVec3, xf: Mat4, other_pos: DVec3, other_xf: Mat4) -> bool {
    (pos - other_pos).length_squared() < 1e-10
        && xf.to_cols_array().iter().zip(other_xf.to_cols_array())
            .all(|(a, b)| (*a - b).abs() < 1e-5)
}

#[derive(Clone,Copy)]
pub enum TextureTarget { Terrain, JunctionRoad, JunctionWalk }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickTarget { Map(i64), Added(usize) }

#[derive(Clone, Copy)]
struct PickCandidate {
    target: PickTarget,
    distance: f64,
    surface: bool,
    anchor: Vec3,
}

fn object_candidate(target: PickTarget, object_type: Option<&crate::scene::ObjectType>, eye: DVec3,
    direction: Vec3, position: DVec3, rotation: Mat4) -> Option<PickCandidate> {
    if direction.length_squared() < 0.5 { return None; }
    if let Some(object_type) = object_type {
        if let Some(distance) = object_type.editor_pick(eye, direction, position, rotation) {
            let anchor = rotation.inverse().transform_point3((eye + direction.as_dvec3() * distance - position).as_vec3());
            return Some(PickCandidate { target, distance, surface: true, anchor });
        }
        if !object_type.meshes.is_empty() || object_type.sco.tree.is_some()
            || !crate::scene::model_light_sources(&object_type.model).is_empty() { return None; }
    }
    // Invisible helpers and mesh-less objects keep the original pivot-based selection.
    let to = position - eye;
    let along = to.dot(direction.as_dvec3());
    if !(0.01..=400.0).contains(&along) { return None; }
    let off = (to - direction.as_dvec3() * along).length() / along;
    (off < 0.35).then_some(PickCandidate { target, distance: off + along * 0.0015, surface: false, anchor: Vec3::ZERO })
}

/// Leaving the editor hides its UI; it must not lose the objects whose helper meshes
/// remain on screen. Resume the same session until a different world is loaded.
pub(crate) fn toggle_session(active: &mut Option<Editor>, paused: &mut Option<Editor>) -> bool {
    if let Some(mut editor) = active.take() {
        editor.catalog = None;
        editor.text_window = None;
        editor.roadside_window = None;editor.sidewalk_window=None;
        editor.placing_asset = None;
        editor.object_stamp = None;
        editor.splines.cancel_generation();
        editor.splines.cancel_connection();
        editor.splines.finish_drag();
        editor.end_object_drag();
        *paused = Some(editor);
        false
    } else {
        *active = Some(paused.take().unwrap_or_default());
        true
    }
}

/// What a key does in the editor.
#[derive(Clone, Copy)]
pub enum Action {
    SplineMode,
    Generate,
    Connect,
    ReplaceConnection,
    FitTerrain,
    SmoothRoad(bool),
    Branch(f64),
    SnapEnd,
    Split,
    SplineUndo,
    PlaceObject,
    Catalog,
    TileWindow,
    ObjectText,
    TerrainMode,
    JunctionWindow,
    RoundaboutWindow,
    RoadsideWindow,
    SidewalkWindow,
    Length(f64),
    Curvature(f64),
    Straight,
    Grade(f64),
    Copy,
    ClipboardCopy,
    Paste,
    RepeatObject,
    PlacementRepeat,
    CancelPlacement,
    Variant,
    Pick,
    NextPick,
    Move(DVec3),
    Turn(f64),
    Tilt(usize,f64),
    Delete,
    Undo,
    Save,
    ReloadMap,
    AuditSplines,
    Leave,
    /// Raise (or lower) the ground under the view by metres.
    Ground(f64),
    Flatten,
    /// Make the brush larger (or smaller) by this factor.
    Brush(f64),
}

impl Editor {
    /// The baseline is needed to apply total object movements exactly once after saving.
    pub(crate) fn audit_saved_sources(&self)->HashMap<(i32,i32),Vec<u8>>{self.saved_tile_sources.lock().unwrap().clone()}

    /// A bounded runtime check for the reported tree changes while editing a road.
    /// Track map identity rather than GPU slot number: tile rebuilds recycle those slots.
    pub(crate) fn trace_trees(&mut self, world: &World, scene: &omsi_render::Scene) {
        if !self.spline_mode || omsi_cfg::env::var_os("OMSI_EDITOR_TREE_TRACE").is_none() {
            self.tree_trace.clear();
            self.tree_trace_at = None;
            self.tree_trace_log_at = None;
            return;
        }
        let now = std::time::Instant::now();
        if self.tree_trace_at.is_some_and(|at| now.duration_since(at).as_secs_f32() < 0.5) { return; }
        self.tree_trace_at = Some(now);
        let types = world.editor_object_types();
        let objects = world.edit_objects.lock().clone();
        let edits = world.object_edits.lock().clone();
        let added_ids: HashSet<i64> = self.added.iter().map(|a| a.id).collect();
        let mut targets = Vec::new();
        for (id, o) in objects {
            if added_ids.contains(&id) || !types.get(&o.sco).is_some_and(|ot| ot.sco.tree.is_some()) { continue; }
            let e = edits.get(&id).copied().unwrap_or_default();
            if e.deleted { continue; }
            let xf = Mat4::from_rotation_z(-(e.turned.to_radians() as f32)) * o.xf;
            targets.push(((o.tile, id), o.sco, o.pos + e.moved, xf, o.instances));
        }
        for a in &self.added {
            if a.deleted || !a._object_type.as_ref().is_some_and(|ot| ot.sco.tree.is_some()) { continue; }
            if let Some(gpu) = &a.gpu {
                let xf = Mat4::from_rotation_z(-((a.base_heading + a.turned).to_radians() as f32)) * a.shape;
                targets.push(((a.tile, a.id), a.sco.clone(), a.base + a.moved, xf, gpu.instances.clone()));
            }
        }
        let mut current = HashMap::new();
        let mut owned = HashSet::new();
        let (mut moved, mut materials, mut uv, mut unexpected, mut duplicate) = (0, 0, 0, 0, 0);
        for (key, sco, pos, xf, instances) in targets {
            for slot in instances {
                let Some(i) = scene.instances.get(slot).filter(|i| i.visible) else { continue; };
                if !owned.insert(slot) { duplicate += 1; }
                if !same_tree_transform(pos, xf, i.origin, i.transform) {
                    unexpected += 1;
                    if unexpected <= 3 {
                        log::warn!("tree audit: unexpected transform, object {} {:?}, GPU slot {}, expected {:?}, actual {:?}", key.1, sco, slot, (pos, xf), (i.origin, i.transform));
                    }
                }
                if let Some(old) = self.tree_trace.get(&key) {
                    if !same_tree_transform(old.pos, old.xf, i.origin, i.transform) {
                        moved += 1;
                        if moved <= 3 {
                            log::warn!("tree audit: changed pose across reload, object {} {:?}, before {:?}, after {:?}",
                                key.1, sco, (old.pos, old.xf), (i.origin, i.transform));
                        }
                    }
                    if old.materials != i.materials { materials += 1; }
                    if old.uv != i.slot_uv { uv += 1; }
                }
                current.insert(key, TreeTracePose {slot, pos: i.origin, xf: i.transform,
                    materials: i.materials.clone(), uv: i.slot_uv.clone()});
            }
        }
        let changed = moved + materials + uv + unexpected + duplicate;
        if changed > 0 || self.tree_trace_log_at.is_none_or(|at| now.duration_since(at).as_secs_f32() >= 5.0) {
            log::info!("tree audit: {} trees, changed poses {}, changed material slots {}, changed UV {}, unexpected transforms {}, duplicate GPU slots {}", current.len(), moved, materials, uv, unexpected, duplicate);
            self.tree_trace_log_at = Some(now);
        }
        // During replacement the old tile remains visible but its editable records
        // temporarily disappear. Keep their history to compare the completed reload.
        // Actual streaming unloads still discard history, so touring the map cannot
        // accumulate a snapshot of every tree ever visited.
        self.tree_trace.retain(|key, old| current.contains_key(key) || scene.instances.get(old.slot)
            .is_some_and(|i| i.visible && same_tree_transform(old.pos, old.xf, i.origin, i.transform)));
        self.tree_trace.extend(current);
    }

    /// Map objects and every object inserted this session share one picking path.
    pub fn pick(&mut self, world: &World, eye: DVec3, forward: Vec3) -> Option<i64> {
        if self.spline_mode {
            self.selected = None;
            self.editing_added = None;
            return self.splines.pick(world, eye, forward);
        }
        let direction = forward.normalize_or_zero();
        // Do not hold an object/edit/type lock while evaluating any geometry.
        let objects = world.edit_objects.lock().clone();
        let edits = world.object_edits.lock().clone();
        let types = world.editor_object_types();
        let added_ids: HashSet<i64> = self.added.iter().map(|a| a.id).collect();
        let mut candidates = Vec::new();
        for (id, object) in &objects {
            if added_ids.contains(id) { continue; }
            let edit = edits.get(id).copied().unwrap_or_default();
            if edit.deleted { continue; }
            let position = object.pos + edit.moved;
            let rotation = Mat4::from_rotation_z(-(edit.turned.to_radians() as f32)) * object.xf;
            if let Some(candidate) = object_candidate(PickTarget::Map(*id), types.get(&object.sco).map(|ot| ot.as_ref()),
                eye, direction, position, rotation) { candidates.push(candidate); }
        }
        for (index, object) in self.added.iter().enumerate() {
            if object.deleted { continue; }
            let position = object.base + object.moved;
            let rotation = Mat4::from_rotation_z(-((object.base_heading + object.turned).to_radians() as f32))*object.shape;
            if let Some(candidate) = object_candidate(PickTarget::Added(index), types.get(&object.sco).map(|ot| ot.as_ref()),
                eye, direction, position, rotation) { candidates.push(candidate); }
        }
        self.pick_candidates(candidates)
    }

    fn pick_candidates(&mut self, mut candidates: Vec<PickCandidate>) -> Option<i64> {
        // An actual surface hit takes priority over an invisible helper's pivot.
        candidates.sort_by(|a, b| b.surface.cmp(&a.surface).then_with(|| a.distance.total_cmp(&b.distance)));
        self.candidates = candidates.into_iter().take(12).collect();
        self.next = 1;
        self.select_candidate(self.candidates.first().copied())
    }

    fn select_candidate(&mut self, candidate: Option<PickCandidate>) -> Option<i64> {
        self.selected = None;
        self.editing_added = None;
        self.drag_offset = None;
        let candidate = candidate?;
        self.pick_anchor = candidate.anchor;
        match candidate.target {
            PickTarget::Map(id) => { self.selected = Some(id); Some(id) }
            PickTarget::Added(index) => {
                let id = self.added.get(index)?.id;
                self.editing_added = Some(index);
                Some(id)
            }
        }
    }

    pub fn next_pick(&mut self) -> Option<i64> {
        if self.spline_mode { return self.splines.next_pick(); }
        if self.candidates.is_empty() {
            return None;
        }
        let candidate = self.candidates[self.next % self.candidates.len()];
        self.next += 1;
        self.select_candidate(Some(candidate))
    }

    fn selected_pose(&self, world: &World) -> Option<(DVec3, Mat4, bool)> {
        if let Some(object) = self.editing_added.and_then(|k| self.added.get(k)) {
            return Some((object.base + object.moved,
                Mat4::from_rotation_z(-((object.base_heading + object.turned).to_radians() as f32))*object.shape, object.deleted));
        }
        let id = self.selected?;
        let object = world.edit_objects.lock().get(&id)?.clone();
        let edit = world.object_edits.lock().get(&id).copied().unwrap_or_default();
        Some((object.pos + edit.moved, Mat4::from_rotation_z(-(edit.turned.to_radians() as f32)) * object.xf, edit.deleted))
    }

    pub fn selection_marker(&self, world: &World) -> Option<DVec3> {
        if self.spline_mode { return None; }
        let (position, rotation, deleted) = self.selected_pose(world)?;
        (!deleted).then(|| position + rotation.transform_point3(self.pick_anchor).as_dvec3() + DVec3::Z * 0.5)
    }

    pub fn begin_object_drag(&mut self, world: &World, ground: DVec3) -> bool {
        let position = self.selected_pose(world).filter(|(_, _, deleted)| !deleted).map(|(position, _, _)| position);
        self.begin_drag_at(position, ground)
    }

    fn begin_drag_at(&mut self, position: Option<DVec3>, ground: DVec3) -> bool {
        self.drag_offset = position.map(|position| position - ground);
        self.drag_offset.is_some()
    }

    pub fn end_object_drag(&mut self) { self.drag_offset = None; }

    fn drag_position(&self, ground: DVec3) -> DVec3 {
        ground + self.drag_offset.unwrap_or(DVec3::ZERO)
    }

    /// The object being edited (a map object or a copy) put where the mouse drags it: its
    /// foot on the ground at `ground`, its turn kept.
    pub fn drag_to(&mut self, world: &World, renderer: &omsi_render::Renderer, scene: &mut omsi_render::Scene, ground: DVec3) -> Option<String> {
        if self.spline_mode { return self.splines.drag_to(world, ground); }
        let ground = self.drag_position(ground);
        if let Some(k) = self.editing_added {
            let a = self.added.get_mut(k)?;
            a.moved = ground - a.base;
            self.place_added(k, world, renderer, scene);
            return Some(self.describe(world));
        }
        let id = self.selected?;
        let (tile, pos) = world.edit_objects.lock().get(&id).map(|o| (o.tile, o.pos))?;
        let mut e = world.object_edits.lock().get(&id).copied().unwrap_or_default();
        e.moved = ground - pos;
        self.tiles.insert(id, tile);
        world.apply_object_edit(renderer, scene, id, e);
        Some(self.describe(world))
    }

    /// What the other players' games need to show the object being edited as it is now
    /// (`all`: every object edited or added this session): LAN commands, see
    /// `App::editor_broadcast`.
    pub fn sync_lines(&self, world: &World, root: &Path, all: bool) -> Vec<String> {
        let mut out = Vec::new();
        let edits = world.object_edits.lock();
        let line = |id: i64, e: &ObjectEdit| format!("objedit {id} {:.3} {:.3} {:.3} {:.2} {}", e.moved.x, e.moved.y, e.moved.z, e.turned, e.deleted as u8);
        if all {
            for (id, e) in edits.iter() {
                out.push(line(*id, e));
            }
        } else if let (None, Some(id)) = (self.editing_added, self.selected) {
            if let Some(e) = edits.get(&id) {
                out.push(line(id, e));
            }
        }
        let added_line = |a: &Added| {
            let rel = a.sco.strip_prefix(root).unwrap_or(&a.sco).to_string_lossy().replace('\\', "/");
            let p = a.base + a.moved;
            format!("objadd {} {:.2} {:.2} {:.2} {:.1} {} {rel}", a.id, p.x, p.y, p.z, a.base_heading + a.turned, a.deleted as u8)
        };
        if all {
            out.extend(self.added.iter().map(added_line));
        } else if let Some(a) = self.editing_added.and_then(|k| self.added.get(k)) {
            out.push(added_line(a));
        }
        out
    }

    /// Change the selected object; returns what to say.
    pub fn apply(&mut self, world: &World, renderer: &omsi_render::Renderer, scene: &mut omsi_render::Scene, action: &Action) -> Option<String> {
        if self.spline_mode { return self.splines.apply(world, action); }
        if let Action::Tilt(axis,delta)=*action {
            let result=(|| {
                let id=self.editing_added.and_then(|i|self.added.get(i)).map(|a|a.id).or(self.selected).ok_or("No object selected")?;
                let (_,_,_,old,_,_)=self.junction_placement(world,id)?;
                let tilt=crate::object_angles::adjusted(old,axis,delta)?;
                self.set_builder_tilt(world,renderer,scene,id,tilt)?;
                self.tilt_baselines.entry(id).or_insert(old);
                Ok::<_,String>(format!("Pitch {:.2}° · Bank {:.2}° · Ctrl+S to save · Reload map for traffic",tilt[0],tilt[1]))
            })();
            return Some(result.unwrap_or_else(|e|e));
        }
        if matches!(action, Action::Undo) {
            let selected_id=self.editing_added.and_then(|k|self.added.get(k)).map(|a|a.id).or(self.selected);
            if let Some((id,tilt))=selected_id.filter(|id|!self.added.iter().any(|a|a.id==*id&&a.deleted)).and_then(|id|self.tilt_baselines.get(&id).copied().map(|tilt|(id,tilt))) {
                return Some(match self.set_builder_tilt(world,renderer,scene,id,tilt) {
                    Ok(())=>{self.tilt_baselines.remove(&id);"Object tilt reset · Ctrl+S to save · Reload map for traffic".into()},Err(e)=>e,
                });
            }
            let selected_id=self.editing_added.and_then(|k|self.added.get(k)).map(|a|a.id).or(self.selected);
            if self.junction_history.last().is_some_and(|h|Some(h.id)==selected_id){
                let h=self.junction_history.last().cloned().unwrap();
                let unchanged=self.junction_placement(world,h.id).is_ok_and(|(_,at,heading,_,_,_)|at.distance(h.at)<1e-6&&(heading-h.heading).abs()<1e-6)
                    &&h.after.iter().all(|(k,s)|world.spline_edits.lock().current(*k).as_ref()==Some(s));
                if !unchanged{return Some("Position or roads changed after the junction edit; undo those changes first".into());}
                return Some(match self.replace_junction(world,renderer,scene,h.id,h.path,h.before,false){Ok(())=>{self.junction_history.pop();"Junction edit and road connections undone · Ctrl+S to save".into()},Err(e)=>e});
            }
            let target = self.editing_added.map(crate::object_text::Target::Added)
                .or_else(|| self.selected.map(crate::object_text::Target::Map));
            if let Some(target) = target {
                let id = self.label_id(target).ok()?;
                if let Some(original) = self.label_history.get(&id).and_then(|h| h.first()).cloned() {
                    if let Err(error) = self.set_labels(world, renderer, scene, target, original, false) { return Some(error); }
                    self.label_history.remove(&id);
                }
            }
        }
        match action {
            Action::Copy => return self.copy(world, renderer, scene),
            Action::Variant => return self.variant(world, renderer, scene),
            _ => {}
        }
        if let Some(k) = self.editing_added {
            let a = self.added.get_mut(k)?;
            match action {
                Action::Move(d) => a.moved += *d,
                Action::Turn(t) => a.turned += *t,
                Action::Delete => a.deleted = !a.deleted,
                Action::Undo => {
                    a.moved = DVec3::ZERO;
                    a.turned = 0.0;
                    a.deleted = false;
                }
                _ => return None,
            }
            self.place_added(k, world, renderer, scene);
            return Some(self.describe(world));
        }
        let id = self.selected?;
        let tile = world.edit_objects.lock().get(&id).map(|o| o.tile)?;
        let mut e = world.object_edits.lock().get(&id).copied().unwrap_or_default();
        match action {
            Action::Move(d) => e.moved += *d,
            Action::Turn(t) => e.turned += *t,
            Action::Delete => e.deleted = !e.deleted,
            Action::Undo => e = ObjectEdit::default(),
            _ => return None,
        }
        self.tiles.insert(id, tile);
        world.apply_object_edit(renderer, scene, id, e);
        Some(self.describe(world))
    }

    pub fn can_copy(&self,world:&World)->bool {self.selected_pose(world).is_some_and(|(_,_,deleted)|!deleted)}

    fn label_id(&self, target: crate::object_text::Target) -> Result<i64, String> {
        match target {
            crate::object_text::Target::Map(id) => Ok(id),
            crate::object_text::Target::Added(i) => self.added.get(i).map(|a| a.id).ok_or_else(|| "Object no longer exists".into()),
        }
    }
    pub fn open_labels(&mut self, world: &World) -> Result<(), String> {
        use crate::object_text::{Target, Window};
        let (target, sco, strings) = if let Some(index) = self.editing_added {
            let a = self.added.get(index).ok_or("Object no longer exists")?;
            if a.deleted { return Err("Restore the deleted object first".into()); }
            (Target::Added(index), a.sco.clone(), a.strings.clone())
        } else {
            let id = self.selected.ok_or("Select the sign first")?;
            let object = world.edit_objects.lock().get(&id).cloned().ok_or("Object no longer loaded")?;
            if world.object_edits.lock().get(&id).is_some_and(|e| e.deleted) { return Err("Restore the deleted object first".into()); }
            let strings = world.object_text_edits.lock().get(&id).cloned().unwrap_or(object.strings);
            (Target::Map(id), object.sco, strings)
        };
        let ot = world.editor_object_type(&sco.to_string_lossy())?;
        let name = sco.file_name().unwrap_or_default().to_string_lossy().into_owned();
        self.text_window = Some(Window::new(target, name, crate::object_text::fields(&ot), strings)?);
        self.placing_asset = None; self.object_stamp = None; self.end_object_drag();
        Ok(())
    }
    pub fn set_labels(&mut self, world: &World, renderer: &omsi_render::Renderer, scene: &mut omsi_render::Scene,
        target: crate::object_text::Target, values: Vec<String>, remember: bool) -> Result<(), String> {
        if values.len() > 4096 || values.iter().any(|s| s.chars().any(char::is_control) || s.chars().count() > 4096) {
            return Err("Text contains invalid characters or is too long".into());
        }
        let id = self.label_id(target)?;
        let old = match target {
            crate::object_text::Target::Added(i) => {
                let a = &mut self.added[i]; let old = a.strings.clone();
                if let Some(gpu) = &mut a.gpu { world.update_editor_helper_labels(renderer, scene, &a.sco, gpu, &values)?; }
                a.strings.clone_from(&values);
                for stamp in [&mut self.last_object, &mut self.object_stamp] {
                    if let Some(stamp) = stamp.as_mut().filter(|s| s.sco == a.sco && s.strings == old) { stamp.strings.clone_from(&values); }
                }
                old
            }
            crate::object_text::Target::Map(id) => {
                let object = world.edit_objects.lock().get(&id).cloned().ok_or("Object no longer loaded")?;
                let old = world.object_text_edits.lock().get(&id).cloned().unwrap_or(object.strings);
                let gpu = world.update_editor_object_labels(renderer, scene, id, &values)?;
                if let Some(previous) = self.label_resources.insert(id, gpu) { world.remove_helper_object(renderer, scene, previous); }
                world.object_text_edits.lock().insert(id, values.clone()); self.tiles.insert(id, object.tile);
                old
            }
        };
        if remember && old != values { self.label_history.entry(id).or_default().push(old); }
        Ok(())
    }
    pub fn undo_labels(&mut self, world: &World, renderer: &omsi_render::Renderer, scene: &mut omsi_render::Scene,
        target: crate::object_text::Target) -> Result<Vec<String>, String> {
        let id = self.label_id(target)?;
        let values = self.label_history.get(&id).and_then(|h| h.last()).cloned().ok_or("No previous text in this session")?;
        self.set_labels(world, renderer, scene, target, values.clone(), false)?;
        self.label_history.get_mut(&id).unwrap().pop();
        Ok(values)
    }

    /// Capture values rather than an instance ID, so deleting/unloading the source is safe.
    pub fn capture_object(&self,world:&World)->Result<ObjectStamp,String> {
        if let Some(a)=self.editing_added.and_then(|i|self.added.get(i)) {
            if a.deleted {return Err("Restore the deleted object first".into());}
            let at=a.base+a.moved;
            let ground=world.ground_terrain(at.x,at.y).unwrap_or(a.ground_height);
            return Ok(ObjectStamp {sco:a.sco.clone(),heading:a.base_heading+a.turned,tilt:a.tilt,
                strings:a.strings.clone(),height_offset:at.z-ground});
        }
        let id=self.selected.ok_or("Select an object first")?;
        let object=world.edit_objects.lock().get(&id).cloned().ok_or("Object no longer loaded")?;
        let edit=world.object_edits.lock().get(&id).copied().unwrap_or_default();
        if edit.deleted {return Err("Restore the deleted object first".into());}
        let src=world.tile_source(object.tile.0,object.tile.1).ok_or("Tile file missing")?;
        let tile=crate::tiles::read_tile(&src,&world.chrono_dirs.read()).ok_or("Cannot read tile")?;
        let record=tile.objects.iter().find(|o|o.id==id).ok_or("Not a standalone map object")?;
        let at=object.pos+edit.moved;let ground=world.ground_terrain(at.x,at.y).unwrap_or(at.z);
        let ot=world.editor_object_type(&object.sco.to_string_lossy())?;
        let height_offset=if ot.sco.absolute_height() {at.z-ground} else {record.pos[2]+edit.moved.z};
        if record.flag>4096 {return Err("Object contains too many text fields".into());}
        let mut strings=world.object_text_edits.lock().get(&id).cloned().unwrap_or_else(|| object.strings.clone());
        strings.resize(strings.len().max(record.flag.max(0) as usize),String::new());
        Ok(ObjectStamp {sco:object.sco,heading:record.rot[0]+edit.turned,tilt:[record.rot[1],record.rot[2]],strings,height_offset})
    }

    pub fn start_object(&mut self,stamp:ObjectStamp) {
        self.text_window=None;
        self.splines.cancel_connection();self.splines.cancel_generation();self.splines.finish_drag();self.end_object_drag();
        self.junction_window=None;self.texture_target=None;self.spline_mode=false;self.terrain.active=false;self.terrain.input=None;self.terrain.cursor=None;self.catalog=None;self.tile_window=None;
        let file=stamp.sco.to_string_lossy().into_owned();
        let name=stamp.sco.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        self.placing_asset=Some(crate::asset_catalog::Asset {kind:crate::asset_catalog::Kind::Object,file,name,path:stamp.sco.clone(),
            category:crate::asset_catalog::Category::Other,groups:String::new()});
        self.object_stamp=Some(stamp);self.align_object=false;
    }

    fn copy(&mut self,world:&World,_renderer:&omsi_render::Renderer,_scene:&mut omsi_render::Scene)->Option<String> {
        match self.capture_object(world) {Ok(stamp)=>{self.clipboard=Some(stamp.clone());self.start_object(stamp);
            Some("Object copied · Click target · Repeat placement for more copies · Esc to finish".into())},Err(e)=>Some(e)}
    }

    /// Insert a chosen scenery object, such as an authored T junction with paths.
    pub fn place_object(&mut self, world: &World, renderer: &omsi_render::Renderer, scene: &mut omsi_render::Scene, sco: PathBuf, at: DVec3, heading: f64) -> String {
        self.place_object_values(world,renderer,scene,sco,at,heading,[0.0;2],Vec::new())
    }

    pub fn place_object_values(&mut self,world:&World,renderer:&omsi_render::Renderer,scene:&mut omsi_render::Scene,sco:PathBuf,at:DVec3,heading:f64,tilt:[f64;2],strings:Vec<String>)->String {
        if omsi_map::world_coordinates() { return "Placement on world-coordinate maps is not supported yet".into(); }
        let size = omsi_map::tile_size();
        let tile = ((at.x / size).floor() as i32, (at.y / size).floor() as i32);
        let Some(source) = world.tile_source(tile.0, tile.1) else { return "Point lies outside the map".into() };
        let Ok(base) = omsi_map::Tile::load(&source) else { return "Cannot read tile".into() };
        if base.version != 0 && base.version < 14 { return "Object placement requires tile version 14".into(); }
        let ot=match world.editor_object_type(&sco.to_string_lossy()) {Ok(ot)=>ot,Err(error)=>{
            log::warn!("editor object placement {}: {error}", sco.display());
            return format!("Object not loaded: {error}");
        }};
        let Some(id) = world.allocate_editor_id() else { return "No free map ID".into() };
        let Some(gpu) = world.add_editor_helper_object(renderer,scene,&sco.to_string_lossy(),at,heading,tilt,&strings) else {
            log::warn!("editor object placement {}: GPU upload failed", sco.display());
            return format!("Object not rendered: {} · See game.log", sco.display());
        };
        world.retain_editor_helper(id);
        self.added.push(Added { template: 0, tile, id, sco, base: at, base_heading: heading,
            moved: DVec3::ZERO, turned: 0.0, deleted: false, standalone: true, shape:crate::scene::editor_helper_shape(&ot.sco,&strings,tilt),
            ground_height:world.ground_terrain(at.x,at.y).unwrap_or(at.z),_object_type:Some(ot),strings,tilt,gpu: Some(gpu) });
        self.editing_added = Some(self.added.len() - 1);
        self.pick_anchor = Vec3::ZERO;
        self.drag_offset = None;
        self.selected = None;
        self.spline_mode = false;
        "Object placed · IJKL to move · N/M to rotate · U/O for height · Ctrl+S to save · Then reload map".into()
    }

    /// Keep the object's ID and native map angles when promoting a loaded builder
    /// object to the existing editable-helper path. No asset file is changed.
    fn set_builder_tilt(&mut self,world:&World,renderer:&omsi_render::Renderer,scene:&mut omsi_render::Scene,id:i64,tilt:[f64;2])->Result<(),String> {
        let (path,at,heading,_,_,_)=self.junction_placement(world,id)?;
        if !builder_asset(&path) {return Err("Tilt controls currently support builder junctions and roundabouts".into());}
        if world.global.world_coordinates {return Err("Object tilt requires a standard OMSI map".into());}
        if self.added.iter().all(|a|a.id!=id) {
            let edit=world.object_edits.lock().get(&id).copied().unwrap_or_default();
            self.replace_junction(world,renderer,scene,id,path,Vec::new(),false)?;
            let a=self.added.iter_mut().find(|a|a.id==id).unwrap();
            a.base=at-edit.moved;a.base_heading=heading-edit.turned;a.moved=edit.moved;a.turned=edit.turned;
        }
        let index=self.added.iter().position(|a|a.id==id).ok_or("Object no longer exists")?;
        let a=&mut self.added[index];
        a.tilt=tilt;a.shape=crate::object_angles::shape(tilt);
        if let Some(gpu)=a.gpu.as_mut() {
            world.reshape_editor_helper(renderer,scene,gpu,a.base+a.moved,a.base_heading+a.turned,a.shape);
        }
        self.editing_added=Some(index);
        Ok(())
    }

    pub(crate) fn junction_placement(&self,world:&World,id:i64)->Result<(PathBuf,DVec3,f64,[f64;2],Vec<String>,(i32,i32)),String>{
        if let Some(a)=self.added.iter().find(|a|a.id==id){
            if a.deleted{return Err("Junction is deleted".into());}
            return Ok((a.sco.clone(),a.base+a.moved,a.base_heading+a.turned,a.tilt,a.strings.clone(),a.tile));
        }
        let o=world.edit_objects.lock().get(&id).cloned().ok_or("Load and select the junction first")?;
        let e=world.object_edits.lock().get(&id).copied().unwrap_or_default();if e.deleted{return Err("Junction is deleted".into());}
        let(tile,_)=world.editor_row_source(o.tile)?;let r=tile.objects.iter().find(|r|r.id==id).ok_or("Not a standalone junction object")?;
        Ok((o.sco,o.pos+e.moved,r.rot[0]+e.turned,[r.rot[1],r.rot[2]],o.strings,o.tile))
    }
    pub(crate) fn replace_junction(&mut self,world:&World,renderer:&omsi_render::Renderer,scene:&mut omsi_render::Scene,id:i64,path:PathBuf,roads:Vec<(crate::spline_editor::Key,omsi_map::MapSpline)>,remember:bool)->Result<(),String>{
        let(old,at,heading,tilt,strings,tile)=self.junction_placement(world,id)?;
        let ot=world.editor_object_type(&path.to_string_lossy())?;
        let before=roads.iter().map(|(key,_)|world.spline_edits.lock().current(*key).map(|s|(*key,s)).ok_or("Connected road no longer available")).collect::<Result<Vec<_>,_>>()?;
        let gpu=world.add_editor_helper_object(renderer,scene,&path.to_string_lossy(),at,heading,tilt,&strings).ok_or("Could not render updated junction")?;
        if let Some(k)=self.added.iter().position(|a|a.id==id){
            let a=&mut self.added[k];if let Some(g)=a.gpu.take(){world.remove_helper_object(renderer,scene,g);}
            a.sco=path;a.gpu=Some(gpu);a.shape=crate::scene::editor_helper_shape(&ot.sco,&strings,tilt);a._object_type=Some(ot);self.editing_added=Some(k);
        }else{
            let mut e=world.object_edits.lock().get(&id).copied().unwrap_or_default();e.deleted=true;
            self.tiles.insert(id,tile);world.apply_object_edit(renderer,scene,id,e);
            self.added.push(Added{template:0,tile,id,sco:path,base:at,base_heading:heading,moved:DVec3::ZERO,turned:0.0,deleted:false,standalone:true,strings:strings.clone(),tilt,shape:crate::scene::editor_helper_shape(&ot.sco,&strings,tilt),ground_height:world.ground_terrain(at.x,at.y).unwrap_or(at.z),_object_type:Some(ot),gpu:Some(gpu)});
            self.editing_added=Some(self.added.len()-1);
        }
        world.retain_editor_helper(id);self.selected=None;
        if remember{self.junction_history.push(JunctionUndo{id,path:old,before,after:roads.clone(),at,heading});}
        self.splines.apply_junction_roads(world,roads);Ok(())
    }

    /// The copy being edited takes the next object type of its folder.
    fn variant(&mut self, world: &World, renderer: &omsi_render::Renderer, scene: &mut omsi_render::Scene) -> Option<String> {
        let k = self.editing_added?;
        let cur = self.added[k].sco.clone();
        let dir = cur.parent()?;
        let mut all: Vec<PathBuf> = omsi_cfg::vfs::list_dir(dir)?.into_iter().map(|(n, _)| dir.join(n)).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sco"))).collect();
        all.sort();
        let i = all.iter().position(|p| p == &cur).map(|i| (i + 1) % all.len()).unwrap_or(0);
        let replacement = all.get(i)?.clone();
        if let Err(error) = world.editor_object_type(&replacement.to_string_lossy()) {
            return Some(format!("Variant not loaded: {error}"));
        }
        if let Some(gpu) = self.added[k].gpu.take() { world.remove_helper_object(renderer, scene, gpu); }
        let ot=world.editor_object_type(&replacement.to_string_lossy()).ok()?;
        self.added[k].shape=crate::scene::editor_helper_shape(&ot.sco,&self.added[k].strings,self.added[k].tilt);
        self.added[k]._object_type=Some(ot);
        self.added[k].sco = replacement;
        self.place_added(k, world, renderer, scene);
        Some(self.describe(world))
    }

    /// Draw copy `k` where it is now.
    fn place_added(&mut self, k: usize, world: &World, renderer: &omsi_render::Renderer, scene: &mut omsi_render::Scene) {
        let a = &mut self.added[k];
        let at=a.base+a.moved;
        if let Some(ground)=world.ground_terrain(at.x,at.y) {a.ground_height=ground;}
        if a.deleted {
            if let Some(gpu) = a.gpu.take() { world.remove_helper_object(renderer, scene, gpu); }
        } else if let Some(gpu) = a.gpu.as_ref() {
            world.move_helper_object(renderer, scene, gpu, a.base + a.moved, a.base_heading + a.turned);
        } else {
            a.gpu = world.add_editor_helper_object(renderer,scene,&a.sco.to_string_lossy(),a.base+a.moved,a.base_heading+a.turned,a.tilt,&a.strings);
        }
    }

    /// The selected object and what has been done to it.
    pub fn describe(&self, world: &World) -> String {
        if self.terrain.active { return self.terrain.describe(); }
        if let Some(asset)=self.placing_asset.as_ref().filter(|a|a.kind==crate::asset_catalog::Kind::Object) {
            let mode = if self.repeat_objects {"Repeat placement ON"} else {"Place once"};
            let values = self.object_stamp.as_ref().map(|s| format!("{} · Rotation {:.1}° · Height {:+.2} m · Pitch {:.2}° · Bank {:.2}°", mode, s.heading, s.height_offset,s.tilt[0],s.tilt[1]))
                .unwrap_or_else(|| mode.to_string());
            return format!("Place object · {} · {} · Click: place · Esc: finish", asset.name, values);
        }
        if self.spline_mode { return self.splines.describe(world); }
        if let Some(a) = self.editing_added.and_then(|k| self.added.get(k)) {
            let name = a.sco.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            return if a.deleted { format!("Object {} · {name} · Deleted · Del restores it", a.id) }
                else { format!("Object {} · {name} · Height {:.2} m · Rotation {:.1}° · Pitch {:.2}° · Bank {:.2}°", a.id, (a.base + a.moved).z, a.base_heading + a.turned,a.tilt[0],a.tilt[1]) };
        }
        let Some(id) = self.selected else { return "No object selected".into() };
        let name = world
            .edit_objects
            .lock()
            .get(&id)
            .and_then(|o| o.sco.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default();
        let e = world.object_edits.lock().get(&id).copied().unwrap_or_default();
        if e.deleted {
            format!("Object {id} · {name} · Deleted · Del restores it")
        } else if e == ObjectEdit::default() {
            format!("Object {id} · {name}")
        } else {
            format!("Object {id} · {name} · Moved {:+.2} / {:+.2} / {:+.2} m · Rotated {:+.1}°", e.moved.x, e.moved.y, e.moved.z, e.turned)
        }
    }

    /// Where the middle of the view meets the ground (within 400 m).
    pub fn aim(world: &World, eye: DVec3, forward: Vec3) -> Option<DVec3> {
        let f = forward.as_dvec3().normalize_or_zero();
        let mut t = 0.5;
        while t < 400.0 {
            let p = eye + f * t;
            if world.ground_terrain(p.x, p.y).is_some_and(|g| p.z <= g) {
                return Some(p);
            }
            t += if t < 50.0 { 0.25 } else { 1.0 };
        }
        None
    }

    /// The keyboard tools and mouse tools share their settings and undo history.
    pub fn ground(&mut self, world: &World, at: Option<DVec3>, action: &Action) -> (String, Vec<(i32, i32)>) {
        use crate::terrain_editor::{Field,Tool};
        if let Action::Brush(f)=action {
            self.terrain.set_value(Field::Radius,self.terrain.radius*f);
            return (format!("Terrain brush: radius {:.1} m",self.terrain.radius),Vec::new());
        }
        let Some(mut at)=at else {return ("Point at the terrain".into(),Vec::new())};
        at.z=world.editor_terrain_height(at.x,at.y).unwrap_or(at.z);
        let (tool,amount)=match action {Action::Ground(d) if *d>=0.0=>(Tool::Raise,*d),
            Action::Ground(d)=>(Tool::Lower,-d),_=>(Tool::Level,1.0)};
        let tiles=self.terrain.once(world,at,tool,amount);
        if tiles.is_empty() {return (self.terrain.message.clone(),tiles);}
        (format!("{} · {} tile(s) · Terrain mode: undo · Ctrl+S to save",tool.title(),tiles.len()),tiles)
    }

    /// Write every tile with edits as a copy under `content` (the map's own folder there),
    /// from the file the game reads it from. Returns the files written.
    pub fn save(&self, world: &World, map_rel: &str, content: &Path, original: &Path) -> Result<Vec<PathBuf>, String> {
        let edits = world.object_edits.lock().clone();
        let mut by_tile: HashMap<(i32, i32), HashMap<i64, ObjectEdit>> = HashMap::new();
        for (id, e) in edits {
            let Some(tile) = self.tiles.get(&id) else { continue };
            by_tile.entry(*tile).or_default().insert(id, e);
        }
        let labels = world.object_text_edits.lock().clone();
        for id in labels.keys() {
            if let Some(tile) = self.tiles.get(id) { by_tile.entry(*tile).or_default(); }
        }
        let mut copies_by_tile: HashMap<(i32, i32), Vec<NewRecord>> = HashMap::new();
        let mut placed_by_tile: HashMap<(i32, i32), Vec<PlacedRecord>> = HashMap::new();
        for a in self.added.iter().filter(|a| a.standalone) {
            let position = a.base + a.moved;
            let size = omsi_map::tile_size();
            let tile = ((position.x / size).floor() as i32, (position.y / size).floor() as i32);
            if tile != a.tile { return Err("New object must remain in its tile; copy and place it in the target tile".into()); }
            // Assets may live in another content root (including mounted packs).
            // Map records keep an OMSI-relative name so a saved map stays portable.
            let mut roots = omsi_cfg::content_roots(); roots.push(world.root.clone());
            let file = roots.iter().filter_map(|root| a.sco.strip_prefix(root).ok().map(|p| (root.components().count(), p)))
                .max_by_key(|(depth, _)| *depth).map(|(_, p)| p).unwrap_or(&a.sco)
                .to_string_lossy().replace('/', "\\");
            placed_by_tile.entry(tile).or_default().push(PlacedRecord { id: a.id, file,
                pos: position - DVec3::new(tile.0 as f64 * size, tile.1 as f64 * size, 0.0),
                heading: a.base_heading + a.turned, tilt:a.tilt,strings:a.strings.clone(),deleted: a.deleted });
            if !a.deleted {
                let ot=world.editor_object_type(&a.sco.to_string_lossy())?;
                if !ot.sco.absolute_height() {
                    let ground=world.ground_terrain(position.x,position.y).unwrap_or(a.ground_height);
                    placed_by_tile.get_mut(&tile).unwrap().last_mut().unwrap().pos.z-=ground;
                }
            }
            by_tile.entry(tile).or_default();
        }
        for a in self.added.iter().filter(|a| !a.deleted && !a.standalone) {
            let file = a.sco.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let offset = a.base + a.moved - world.edit_objects.lock().get(&a.template).map(|o| o.pos).unwrap_or(a.base);
            copies_by_tile.entry(a.tile).or_default().push(NewRecord { template: a.template, id: a.id, file, moved: offset, turned: a.base_heading + a.turned - template_heading(world, a.template).unwrap_or(a.base_heading) });
            by_tile.entry(a.tile).or_default();
        }
        let roadside_edits = world.roadside_edits.lock();
        for tile in &roadside_edits.save_tiles { by_tile.entry(*tile).or_default(); }
        let spline_edits = world.spline_edits.lock().clone();
        for (tile, _) in spline_edits.changed.keys().chain(spline_edits.added.keys()) {
            by_tile.entry(*tile).or_default();
        }
        let map_dir = Path::new(map_rel).parent().unwrap_or(Path::new(""));
        let mut roundabout_rules:HashMap<(i32,i32),Vec<(i64,Option<String>)>>=HashMap::new();
        for a in self.added.iter().filter(|a|a.standalone) {
            let rules=if a.deleted {None}else{crate::junction_builder::roundabout_rules(&a.sco)?};
            roundabout_rules.entry(a.tile).or_default().push((a.id,rules));
        }
        let mut written = Vec::new();
        for ((tx, ty), edits) in by_tile {
            let src = world.tile_source(tx, ty).ok_or_else(|| format!("tile ({tx}, {ty}) is not in the map"))?;
            // (a LAN host's map is sealed: never written out in plain form, edited or not)
            if omsi_cfg::vfs::is_sealed(&src) {
                return Err(format!("{} is the LAN host's map: not written", src.display()));
            }
            let name = src.file_name().ok_or("tile without a name")?.to_owned();
            let out = content.join(map_dir).join(&name);
            // never into the installation itself
            if let (Ok(o), Ok(r)) = (out.parent().map(|p| p.to_path_buf()).unwrap_or_default().canonicalize().or_else(|_| Ok::<_, std::io::Error>(out.clone())), original.canonicalize()) {
                if o.starts_with(&r) {
                    return Err(format!("{} lies in the original installation: not written", out.display()));
                }
            }
            let cached = self.saved_tile_sources.lock().unwrap().get(&(tx, ty)).cloned();
            let bytes = match cached {
                Some(bytes) => bytes,
                None => omsi_cfg::vfs::read(&src).map_err(|e| format!("{}: {e}", src.display()))?,
            };
            let (text, enc) = decode(&bytes);
            let tile_labels: HashMap<_, _> = labels.iter().filter(|(id, _)| self.tiles.get(*id) == Some(&(tx,ty)))
                .map(|(id, strings)| (*id, strings.clone())).collect();
            let (text, l) = rewrite_labels(&text, &tile_labels)?;
            let (new_text, n) = rewrite_tile(&text, &edits);
            let (new_text, c) = add_copies(&new_text, copies_by_tile.get(&(tx, ty)).map(|v| v.as_slice()).unwrap_or(&[]));
            let (new_text, p) = rewrite_placed(&new_text, placed_by_tile.get(&(tx, ty)).map(|v| v.as_slice()).unwrap_or(&[]))?;
            let new_text=crate::junction_builder::rewrite_roundabout_rules(&new_text,roundabout_rules.get(&(tx,ty)).map(Vec::as_slice).unwrap_or(&[]))?;
            let additions: Vec<i64> = spline_edits.added.keys().filter(|(tile, _)| *tile == (tx, ty)).map(|(_, id)| *id).collect();
            let (new_text, s) = crate::spline_editor::rewrite(&new_text, &spline_edits.for_tile((tx, ty)), &additions)?;
            let (new_text, rows) = crate::roadside_objects::rewrite(&new_text, (tx, ty), &roadside_edits)?;
            let n = n + c + p + s + l + rows;
            if n == 0 {
                return Err(format!("the objects edited were not found in {}", src.display()));
            }
            std::fs::create_dir_all(out.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
            let data = encode(&new_text, enc);
            save_copy(&out, &data)?;
            self.saved_tile_sources.lock().unwrap().entry((tx, ty)).or_insert(bytes);
            for added in self.added.iter().filter(|a| a.tile == (tx, ty)) { world.retain_editor_helper(added.id); }
            log::info!("object editor: {} objects of tile ({tx}, {ty}) changed, written to {}", n, out.display());
            written.push(out);
        }
        // the ground the brush shaped: each tile's .map.terrain
        let terrains = world.terrain_edits.lock().clone();
        for ((tx, ty), t) in terrains {
            let src = world.tile_source(tx, ty).ok_or_else(|| format!("tile ({tx}, {ty}) is not in the map"))?;
            // (a LAN host's map is sealed: never written out in plain form, edited or not)
            if omsi_cfg::vfs::is_sealed(&src) {
                return Err(format!("{} is the LAN host's map: not written", src.display()));
            }
            let name = format!("{}.terrain", src.file_name().ok_or("tile without a name")?.to_string_lossy());
            let out = content.join(map_dir).join(&name);
            if let (Some(dir), Ok(r)) = (out.parent(), original.canonicalize()) {
                if dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()).starts_with(&r) {
                    return Err(format!("{} lies in the original installation: not written", out.display()));
                }
            }
            std::fs::create_dir_all(out.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
            save_copy(&out, &t.to_bytes())?;
            log::info!("map editor: the ground of tile ({tx}, {ty}) written to {}", out.display());
            written.push(out);
        }
        written.extend(crate::ground_paint::save(world,map_rel,content,original)?);
        Ok(written)
    }
}

/// Keep the previous mod tile as a backup and replace it only after all bytes were written.
pub(crate) fn save_copy(out: &Path, data: &[u8]) -> Result<(), String> {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_nanos();
    let name = out.file_name().ok_or("Tile has no filename")?.to_string_lossy();
    if out.exists() {
        let backup = out.with_file_name(format!("{name}.before-editor-{stamp}"));
        std::fs::copy(out, &backup).map_err(|e| format!("Backup {}: {e}", backup.display()))?;
    }
    let temp = out.with_file_name(format!("{name}.editor-{stamp}.tmp"));
    let result = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
        f.write_all(data)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&temp, out)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(format!("{}: {e}", out.display()));
    }
    Ok(())
}

/// Resolve the existing parent before writing, also when the final path is new.
pub(crate) fn protect_output(out:&Path,original:&Path) -> Result<(),String> {
    let mut ancestor=out;
    while !ancestor.exists() {ancestor=ancestor.parent().ok_or("Output folder missing")?;}
    let resolved=ancestor.canonicalize().map_err(|e|e.to_string())?;
    if original.canonicalize().is_ok_and(|root|resolved.starts_with(root)) {
        return Err(format!("{} lies in the original installation",out.display()));
    }
    Ok(())
}

/// The heading of map object `id` as it stands now (its edits included).
fn template_heading(world: &World, id: i64) -> Option<f64> {
    let objects = world.edit_objects.lock();
    let o = objects.get(&id)?;
    let e = world.object_edits.lock().get(&id).copied().unwrap_or_default();
    let f = o.xf.transform_vector3(Vec3::Y);
    Some((f.x as f64).atan2(f.y as f64).to_degrees() + e.turned)
}

/// A new object for the tile file: a copy of `template`'s record with its own id, another
/// file name of the same folder (or the same), moved and turned from the template.
struct PlacedRecord {
    id: i64,
    file: String,
    pos: DVec3,
    heading: f64,
    tilt:[f64;2],
    strings:Vec<String>,
    deleted: bool,
}

/// Only directly inserted objects are replaced; all other records remain byte-for-byte.
fn rewrite_placed(text: &str, records: &[PlacedRecord]) -> Result<(String, usize), String> {
    if records.is_empty() { return Ok((text.into(), 0)); }
    fn body(line: &str) -> &str { line.trim_end_matches(['\r', '\n']) }
    fn record(r: &PlacedRecord, eol: &str) -> String {
        ["[object]".into(), "0".into(), r.file.clone(), r.id.to_string(), num(r.pos.x), num(r.pos.y),
            num(r.pos.z), num(r.heading), num(r.tilt[0]), num(r.tilt[1]), r.strings.len().to_string()]
            .into_iter().chain(r.strings.iter().cloned()).collect::<Vec<_>>().join(eol) + eol
    }
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let mut out = String::new();
    let mut found = std::collections::HashSet::new();
    let mut i = 0;
    while i < lines.len() {
        let object = body(lines[i]).trim().eq_ignore_ascii_case("[object]");
        let id = lines.get(i + 3).and_then(|l| body(l).trim().parse::<i64>().ok());
        if object && records.iter().any(|r| Some(r.id) == id) {
            let id = id.unwrap();
            if !found.insert(id) { return Err(format!("Object {id} exists more than once; nothing saved")); }
            let labels=lines.get(i+10).and_then(|l|body(l).trim().parse::<usize>().ok())
                .filter(|n|*n<=4096 && i+11+*n<=lines.len()).ok_or_else(||format!("Object {id}: damaged text fields"))?;
            if let Some(r) = records.iter().find(|r| r.id == id && !r.deleted) { out.push_str(&record(r, eol)); }
            i += 11+labels;
        } else if object {
            if let Some(end) = object_record_end(&lines, i) {
                for line in &lines[i..end] { out.push_str(line); } i = end;
            } else { out.push_str(lines[i]); i += 1; }
        } else { out.push_str(lines[i]); i += 1; }
    }
    for r in records.iter().filter(|r| !r.deleted && !found.contains(&r.id)) {
        if !out.ends_with('\n') { out.push_str(eol); }
        out.push_str(eol);
        out.push_str(&record(r, eol));
    }
    Ok((out, records.len()))
}

pub struct NewRecord {
    pub template: i64,
    pub id: i64,
    pub file: String,
    pub moved: DVec3,
    pub turned: f64,
}

/// The tile file with `copies` added, each record after its template's. Returns the text
/// and how many were added.
pub fn add_copies(text: &str, copies: &[NewRecord]) -> (String, usize) {
    if copies.is_empty() {
        return (text.to_string(), 0);
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let body = |l: &str| l.trim_end_matches(['\r', '\n']).to_string();
    let ending = |l: &str| l[l.trim_end_matches(['\r', '\n']).len()..].to_string();
    let mut out = String::with_capacity(text.len() + copies.len() * 200);
    let mut added = 0;
    let mut i = 0;
    while i < lines.len() {
        let is_object = body(lines[i]).trim().eq_ignore_ascii_case("[object]");
        let id = lines.get(i + 3).and_then(|l| body(l).trim().parse::<i64>().ok());
        let mine: Vec<&NewRecord> = if is_object { copies.iter().filter(|c| Some(c.template) == id).collect() } else { Vec::new() };
        if mine.is_empty() {
            out.push_str(lines[i]);
            i += 1;
            continue;
        }
        // the template's record, up to the next keyword
        let start = i;
        i += 1;
        while i < lines.len() && !body(lines[i]).trim_start().starts_with('[') {
            i += 1;
        }
        let record = &lines[start..i];
        for l in record {
            out.push_str(l);
        }
        let eol = ending(record[0]);
        // (a blank line between records, as the editor writes them)
        if !record.last().map(|l| body(l).trim().is_empty()).unwrap_or(false) {
            out.push_str(&eol);
        }
        for c in mine {
            for (k, l) in record.iter().enumerate() {
                let b = body(l);
                let new = match k {
                    // the file: the copy's name in the template's folder
                    2 => match b.rfind(['\\', '/']) {
                        Some(p) => format!("{}{}", &b[..=p], c.file),
                        None => c.file.clone(),
                    },
                    3 => c.id.to_string(),
                    4..=7 => match b.trim().parse::<f64>() {
                        Ok(v) => num(v + [c.moved.x, c.moved.y, c.moved.z, c.turned][k - 4]),
                        Err(_) => b.clone(),
                    },
                    _ => b.clone(),
                };
                out.push_str(&new);
                out.push_str(&ending(l));
            }
            if !record.last().map(|l| body(l).trim().is_empty()).unwrap_or(false) {
                out.push_str(&eol);
            }
            added += 1;
        }
    }
    (out, added)
}

/// How a tile file is written: OMSI's editor saves UTF-16 (little endian, with its byte
/// order mark); hand-made ones are ASCII or Latin-1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Encoding {
    Utf8,
    Latin1,
    Utf16Le,
}

pub(crate) fn decode(bytes: &[u8]) -> (String, Encoding) {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return (String::from_utf16_lossy(&units), Encoding::Utf16Le);
    }
    match String::from_utf8(bytes.to_vec()) {
        Ok(t) => (t, Encoding::Utf8),
        Err(_) => (bytes.iter().map(|&b| b as char).collect(), Encoding::Latin1),
    }
}

pub(crate) fn encode(text: &str, enc: Encoding) -> Vec<u8> {
    if enc == Encoding::Latin1 && text.chars().any(|c| c as u32 > 255) {
        return encode(text, Encoding::Utf16Le);
    }
    match enc {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Latin1 => text.chars().map(|c| c as u32 as u8).collect(),
        Encoding::Utf16Le => [0xFF, 0xFE].into_iter().chain(text.encode_utf16().flat_map(|u| u.to_le_bytes())).collect(),
    }
}

/// A number as a tile file writes it.
fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

/// Honour the count of trailing strings, including empty lines and keyword-like
/// labels. A sign's text must never become a new tile keyword.
pub(crate) fn object_record_end(lines: &[&str], start: usize) -> Option<usize> {
    let count = lines.get(start + 10)?.trim().parse::<usize>().ok()?;
    if count > 4096 { return None; }
    let end = start.checked_add(11 + count)?;
    (end <= lines.len()).then_some(end)
}
fn rewrite_labels(text: &str, labels: &HashMap<i64, Vec<String>>) -> Result<(String, usize), String> {
    if labels.is_empty() { return Ok((text.into(), 0)); }
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let mut out = String::with_capacity(text.len()); let mut changed = HashSet::new(); let mut i = 0;
    while i < lines.len() {
        if !lines[i].trim().eq_ignore_ascii_case("[object]") { out.push_str(lines[i]); i += 1; continue; }
        let id = lines.get(i + 3).and_then(|s| s.trim().parse::<i64>().ok());
        let end = object_record_end(&lines, i);
        if let Some(values) = id.and_then(|id| labels.get(&id)) {
            let end = end.ok_or_else(|| format!("Invalid object record for text field {}", id.unwrap()))?;
            let eol = if lines[i].ends_with("\r\n") { "\r\n" } else { "\n" };
            for line in &lines[i..i+10] { out.push_str(line); }
            out.push_str(&values.len().to_string()); out.push_str(eol);
            for value in values { out.push_str(value); out.push_str(eol); }
            changed.insert(id.unwrap()); i = end;
        } else if let Some(end) = end {
            for line in &lines[i..end] { out.push_str(line); } i = end;
        } else { out.push_str(lines[i]); i += 1; }
    }
    if changed.len() != labels.len() { return Err("An object with text is missing from the tile file; not saved".into()); }
    Ok((out, changed.len()))
}

/// The tile file with the edits applied to its `[object]` records (by map id): a moved or
/// turned object gets its position (x, y, height over the ground) and heading changed, a
/// deleted one loses its record. Everything else stays as it was, line endings included.
/// Returns the text and how many records changed.
pub fn rewrite_tile(text: &str, edits: &HashMap<i64, ObjectEdit>) -> (String, usize) {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let body = |l: &str| l.trim_end_matches(['\r', '\n']).to_string();
    let ending = |l: &str| l[l.trim_end_matches(['\r', '\n']).len()..].to_string();
    let mut out = String::with_capacity(text.len());
    let mut changed = 0;
    let mut i = 0;
    while i < lines.len() {
        let is_object = body(lines[i]).trim().eq_ignore_ascii_case("[object]");
        let id = lines.get(i + 3).and_then(|l| body(l).trim().parse::<i64>().ok());
        let edit = if is_object { id.and_then(|id| edits.get(&id)) } else { None };
        let Some(e) = edit else {
            if is_object {
                if let Some(end) = object_record_end(&lines, i) {
                    for line in &lines[i..end] { out.push_str(line); } i = end; continue;
                }
            }
            out.push_str(lines[i]);
            i += 1;
            continue;
        };
        changed += 1;
        if e.deleted {
            if let Some(end) = object_record_end(&lines, i) { i = end; continue; }
            // the record up to the next keyword
            i += 1;
            while i < lines.len() && !body(lines[i]).trim_start().starts_with('[') {
                i += 1;
            }
            continue;
        }
        // [object], 0, file, id, x, y, z, heading, …
        for k in 0..4 {
            out.push_str(lines[i + k]);
        }
        let deltas = [e.moved.x, e.moved.y, e.moved.z, e.turned];
        for (k, d) in deltas.iter().enumerate() {
            let Some(l) = lines.get(i + 4 + k) else { break };
            match body(l).trim().parse::<f64>() {
                Ok(v) if *d != 0.0 => {
                    out.push_str(&num(v + d));
                    out.push_str(&ending(l));
                }
                _ => out.push_str(l),
            }
        }
        if let Some(end) = object_record_end(&lines, i) {
            for line in &lines[i+8..end] { out.push_str(line); } i = end;
        } else { i += 8; }
    }
    (out, changed)
}

/// Assets produced by either of the road junction builders.
pub fn builder_asset(path:&Path)->bool {
    path.file_name().is_some_and(|n|n=="junction.sco")
        && path.parent().is_some_and(|dir|dir.join("junction.junction.json").is_file())
}
pub fn action_for_mode(code:winit::keyboard::KeyCode,shift:bool,ctrl:bool,yaw:f64,spline:bool)->Option<Action> {
    if !spline && matches!(code,winit::keyboard::KeyCode::Comma|winit::keyboard::KeyCode::Period) {return None;}
    if !spline {if let Some(step)=crate::object_angles::key(code,shift,ctrl) {
        return Some(match step {crate::object_angles::Step::Turn(t)=>Action::Turn(t),crate::object_angles::Step::Tilt(axis,d)=>Action::Tilt(axis,d)});
    }}
    action_for(code,shift,ctrl,yaw)
}

/// Resolve the original spline and general editor shortcuts.
pub fn action_for(code: winit::keyboard::KeyCode, shift: bool, ctrl: bool, yaw: f64) -> Option<Action> {
    use winit::keyboard::KeyCode as K;
    let step = if shift { 0.05 } else { 0.5 };
    let turn = if shift { 0.5 } else { 5.0 };
    let (s, c) = yaw.to_radians().sin_cos();
    let fwd = DVec3::new(s, c, 0.0) * step;
    let right = DVec3::new(c, -s, 0.0) * step;
    Some(match code {
        K::Enter | K::NumpadEnter => Action::Pick,
        K::KeyC if !ctrl => Action::Copy,
        K::KeyC if ctrl => Action::ClipboardCopy,
        K::KeyV if ctrl => Action::Paste,
        K::KeyR if ctrl => Action::RepeatObject,
        K::KeyV if !ctrl => Action::Variant,
        K::Tab => Action::NextPick,
        K::KeyI => Action::Move(fwd),
        K::KeyK => Action::Move(-fwd),
        K::KeyL => Action::Move(right),
        K::KeyJ => Action::Move(-right),
        K::KeyO => Action::Move(DVec3::Z * step),
        K::KeyU => Action::Move(-DVec3::Z * step),
        K::KeyM => Action::Turn(turn),
        K::KeyN => Action::Turn(-turn),
        K::Delete => Action::Delete,
        K::Backspace => Action::Undo,
        K::KeyS if ctrl => Action::Save,
        K::F7 if !ctrl => Action::Split,
        K::KeyT if !ctrl => Action::SplineMode,
        K::KeyG if !ctrl && shift => Action::Generate,
        K::KeyG if ctrl && !shift => Action::ReplaceConnection,
        K::KeyG if !ctrl => Action::Connect,
        K::F6 if !ctrl => Action::FitTerrain,
        K::F8 if !ctrl => Action::SmoothRoad(shift),
        K::KeyR if !ctrl => Action::Branch(if shift { -1.0 } else { 1.0 }),
        K::KeyH if !ctrl => Action::SnapEnd,
        K::KeyX if !ctrl => Action::PlaceObject,
        K::KeyP if !ctrl => Action::Catalog,
        K::KeyZ if ctrl => Action::SplineUndo,
        K::Equal | K::NumpadAdd => Action::Length(if shift { 0.1 } else { 1.0 }),
        K::Minus | K::NumpadSubtract => Action::Length(if shift { -0.1 } else { -1.0 }),
        K::Comma => Action::Curvature(if shift { -0.0001 } else { -0.001 }),
        K::Period => Action::Curvature(if shift { 0.0001 } else { 0.001 }),
        K::KeyB if !ctrl => Action::Straight,
        K::Home => Action::Grade(if shift { 0.05 } else { 0.5 }),
        K::End => Action::Grade(if shift { -0.05 } else { -0.5 }),
        K::PageUp => Action::Ground(if shift { 0.05 } else { 0.25 }),
        K::PageDown => Action::Ground(if shift { -0.05 } else { -0.25 }),
        K::KeyF if !ctrl => Action::Flatten,
        K::BracketRight => Action::Brush(1.25),
        K::BracketLeft => Action::Brush(0.8),
        K::Escape => Action::Leave,
        _ => return None,
    })
}

/// Repeat suppression follows the action mapping, rather than swallowing camera
/// letters that no longer belong to an editor operation.
pub(crate) fn one_shot_key(code: winit::keyboard::KeyCode, shift: bool, ctrl: bool) -> bool {
    action_for(code, shift, ctrl, 0.0).is_some_and(|action| matches!(action,
        Action::FitTerrain | Action::SmoothRoad(_) | Action::Catalog | Action::PlaceObject
        | Action::Generate | Action::Connect | Action::ReplaceConnection | Action::Copy
        | Action::ClipboardCopy | Action::Paste | Action::RepeatObject | Action::PlacementRepeat | Action::CancelPlacement
        | Action::Branch(_) | Action::Split | Action::Save | Action::SplineMode
        | Action::Straight | Action::SplineUndo | Action::Pick | Action::Delete
        | Action::Undo | Action::Leave))
}

/// The frame loop must respect editor ownership too: a key consumed by on_key
/// remains in the held-key set until release, so reading that set directly flies
/// the camera at the same time (for example Ctrl+S).
pub(crate) fn camera_key_pressed(keys: &hashbrown::HashSet<winit::keyboard::KeyCode>, editor: Option<&Editor>, code: winit::keyboard::KeyCode) -> bool {
    use winit::keyboard::KeyCode as K;
    if !keys.contains(&code) { return false; }
    let Some(editor) = editor else { return true; };
    if editor.sidewalk_window.as_ref().is_some_and(|w| w.input.is_some()) || editor.catalog.is_some() || editor.tile_window.is_some() || editor.text_window.is_some() || editor.junction_window.is_some() || editor.terrain.input.is_some() || editor.roadside_window.as_ref().is_some_and(|w| w.input.is_some()) { return false; }
    let shift = keys.contains(&K::ShiftLeft) || keys.contains(&K::ShiftRight);
    let ctrl = keys.contains(&K::ControlLeft) || keys.contains(&K::ControlRight);
    action_for(code, shift, ctrl, 0.0).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn added_object(id: i64) -> Added {
        Added { template: 0, tile: (0, 0), id, sco: PathBuf::from("Sceneryobjects/junction.sco"),
            base: DVec3::new(10.0, 20.0, 1.5), base_heading: 0.0,
            moved: DVec3::ZERO, turned: 0.0, deleted: false, standalone: true,strings:Vec::new(),tilt:[0.0;2],shape:Mat4::IDENTITY,ground_height:0.0,_object_type:None,gpu: None }
    }

    fn surface_candidate(target: PickTarget, distance: f64) -> PickCandidate {
        PickCandidate { target, distance, surface: true, anchor: Vec3::new(35.0, 0.0, 0.0) }
    }

    #[test]
    fn all_inserted_objects_can_be_reselected_and_tab_cycles_between_new_and_map_objects() {
        let mut editor = Editor::default();
        editor.added = vec![added_object(101), added_object(102)];
        editor.editing_added = Some(1);
        assert_eq!(editor.pick_candidates(vec![surface_candidate(PickTarget::Added(0), 20.0),
            surface_candidate(PickTarget::Map(7), 30.0), surface_candidate(PickTarget::Added(1), 40.0)]), Some(101));
        assert_eq!(editor.editing_added, Some(0));
        assert_eq!(editor.selected, None);
        assert_eq!(editor.next_pick(), Some(7));
        assert_eq!(editor.selected, Some(7));
        assert_eq!(editor.editing_added, None);
        assert_eq!(editor.next_pick(), Some(102));
        assert_eq!(editor.editing_added, Some(1));
        assert_eq!(editor.selected, None);
        assert_eq!(editor.next_pick(), Some(101));
        assert_eq!(editor.pick_candidates(Vec::new()), None);
        assert_eq!(editor.editing_added, None);
        assert_eq!(editor.selected, None);
    }

    #[test]
    fn an_actual_junction_surface_takes_priority_over_a_nearby_helpers_pivot() {
        let mut editor = Editor::default();
        editor.added.push(added_object(101));
        let helper = PickCandidate { target: PickTarget::Map(7), distance: 0.1, surface: false, anchor: Vec3::ZERO };
        assert_eq!(editor.pick_candidates(vec![helper, surface_candidate(PickTarget::Added(0), 80.0)]), Some(101));
        assert_eq!(editor.editing_added, Some(0));
    }

    #[test]
    fn dragging_a_large_junction_preserves_the_grab_point_and_height_offset() {
        let mut editor = Editor::default();
        let original = DVec3::new(10.0, 20.0, 1.5);
        let clicked = DVec3::new(55.0, 25.0, 0.0);
        assert!(editor.begin_drag_at(Some(original), clicked));
        assert_eq!(editor.drag_position(clicked), original);
        assert_eq!(editor.drag_position(clicked + DVec3::new(3.0, -2.0, 0.5)), DVec3::new(13.0, 18.0, 2.0));
        editor.end_object_drag();
        assert_eq!(editor.drag_offset, None);
        assert!(!editor.begin_drag_at(None, clicked));
    }

    #[test]
    fn closing_and_reopening_the_editor_retains_new_objects_and_their_edits() {
        let mut editor = Editor::default();
        let mut object = added_object(101);
        object.moved = DVec3::new(2.0, -4.0, 0.5);
        object.turned = 35.0;
        object.deleted = true;
        editor.added.push(object);
        editor.editing_added = Some(0);
        editor.drag_offset = Some(DVec3::X);
        editor.placing_asset = Some(crate::asset_catalog::Asset { kind: crate::asset_catalog::Kind::Object,category:crate::asset_catalog::Category::Other,groups:String::new(),
            name: "test".into(), file: "Sceneryobjects/test.sco".into(), path: PathBuf::from("test.sco") });
        let mut active = Some(editor);
        let mut paused = None;
        assert!(!toggle_session(&mut active, &mut paused));
        assert!(active.is_none());
        assert!(paused.as_ref().unwrap().placing_asset.is_none());
        assert!(paused.as_ref().unwrap().drag_offset.is_none());
        assert!(toggle_session(&mut active, &mut paused));
        assert!(paused.is_none());
        let resumed = active.unwrap();
        assert_eq!(resumed.editing_added, Some(0));
        assert_eq!(resumed.added[0].id, 101);
        assert_eq!(resumed.added[0].moved, DVec3::new(2.0, -4.0, 0.5));
        assert_eq!(resumed.added[0].turned, 35.0);
        assert!(resumed.added[0].deleted);
    }

    #[test]
    fn camera_letters_remain_available_in_both_editor_modes_and_with_shift() {
        use winit::keyboard::KeyCode as K;
        let keys = hashbrown::HashSet::from([K::KeyA, K::KeyS, K::KeyQ, K::KeyE, K::KeyW, K::KeyD]);
        let mut editor = Editor::default();
        for spline_mode in [false, true] {
            editor.spline_mode = spline_mode;
            for shift in [false, true] {
                let mut held = keys.clone();
                if shift { held.insert(K::ShiftRight); }
                for code in keys.iter().copied() {
                    assert!(camera_key_pressed(&held, Some(&editor), code));
                    assert!(action_for(code, shift, false, 0.0).is_none());
                    assert!(!one_shot_key(code, shift, false));
                }
            }
        }
        assert!(camera_key_pressed(&keys, None, K::KeyA));
        assert!(camera_key_pressed(&keys, None, K::KeyS));
        assert!(!camera_key_pressed(&keys, Some(&editor), K::ArrowUp));
        let save = hashbrown::HashSet::from([K::KeyS, K::ControlRight]);
        assert!(!camera_key_pressed(&save, Some(&editor), K::KeyS));
        assert!(matches!(action_for(K::KeyS, false, true, 0.0), Some(Action::Save)));
        assert!(one_shot_key(K::KeyS, false, true));
    }

    #[test]
    fn terrain_fit_and_split_use_function_keys_and_camera_release_is_respected() {
        use winit::keyboard::KeyCode as K;
        assert!(matches!(action_for(K::F6, false, false, 0.0), Some(Action::FitTerrain)));
        assert!(matches!(action_for(K::F7, false, false, 0.0), Some(Action::Split)));
        assert!(one_shot_key(K::F6, false, false));
        assert!(one_shot_key(K::F7, false, false));
        let editor = Editor::default();
        let mut held = hashbrown::HashSet::from([K::KeyA, K::KeyS]);
        held.remove(&K::KeyA);
        assert!(!camera_key_pressed(&held, Some(&editor), K::KeyA));
        assert!(camera_key_pressed(&held, Some(&editor), K::KeyS));
        held.remove(&K::KeyS);
        assert!(!camera_key_pressed(&held, Some(&editor), K::KeyS));
    }

    #[test]
    fn smoothing_shortcuts_have_no_camera_letter_binding() {
        use winit::keyboard::KeyCode as K;
        assert!(matches!(action_for(K::F8, false, false, 0.0), Some(Action::SmoothRoad(false))));
        assert!(matches!(action_for(K::F8, true, false, 0.0), Some(Action::SmoothRoad(true))));
        assert!(action_for(K::KeyQ, false, false, 0.0).is_none());
        assert!(action_for(K::KeyE, false, false, 0.0).is_none());
        let mut keys = hashbrown::HashSet::from([K::F8]);
        let editor = Editor::default();
        assert!(!camera_key_pressed(&keys, Some(&editor), K::KeyQ));
        assert!(!camera_key_pressed(&keys, Some(&editor), K::KeyE));
        keys.insert(K::KeyQ);
        assert!(camera_key_pressed(&keys, Some(&editor), K::KeyQ));
        keys.remove(&K::KeyQ);
        assert!(!camera_key_pressed(&keys, Some(&editor), K::KeyQ));
    }

    #[test]
    fn terrain_number_entry_captures_camera_keys_until_committed() {
        use winit::keyboard::KeyCode as K;
        let mut editor=Editor::default();editor.terrain.active=true;
        let keys=hashbrown::HashSet::from([K::KeyW,K::KeyA,K::KeyS,K::KeyD,K::KeyQ,K::KeyE]);
        for code in keys.iter().copied() {assert!(camera_key_pressed(&keys,Some(&editor),code));}
        editor.terrain.edit(crate::terrain_editor::Field::Height);
        for code in keys.iter().copied() {assert!(!camera_key_pressed(&keys,Some(&editor),code));}
        editor.terrain.input.as_mut().unwrap().text="42,5".into();
        assert!(editor.terrain.commit_input());assert_eq!(editor.terrain.height,42.5);
        for code in keys.iter().copied() {assert!(camera_key_pressed(&keys,Some(&editor),code));}
    }

    #[test]
    fn saving_again_after_the_overlay_becomes_the_source_does_not_accumulate_object_edits() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("omsi-repeat-save-{}-{stamp}", std::process::id()));
        let original = dir.join("original"); let content = dir.join("mod"); let map = original.join("maps/Test");
        std::fs::create_dir_all(&map).unwrap();
        std::fs::write(map.join("global.cfg"), "[map]\n0\n0\ntile_0_0.map\n").unwrap();
        let source = "[version]\n14\n[terrain]\n[object]\n0\nSceneryobjects\\tree.sco\n7\n20\n40\n1.25\n15\n0\n0\n0\n";
        std::fs::write(map.join("tile_0_0.map"), source).unwrap();
        let world = World::open(&original, &map.join("global.cfg"), 20000101).unwrap();
        let mut editor = Editor::default(); editor.tiles.insert(7, (0, 0));
        world.object_edits.lock().insert(7, ObjectEdit { moved: DVec3::new(2.0, 3.0, 0.5), turned: 5.0, deleted: false });
        editor.save(&world, "maps/Test/global.cfg", &content, &original).unwrap();
        let saved = content.join("maps/Test/tile_0_0.map");
        let first = std::fs::read(&saved).unwrap();
        // Equivalent to the VFS choosing the saved mod copy on subsequent reads.
        world.register_editor_tile((0, 0), 0, saved.clone());
        for _ in 0..3 {
            editor.save(&world, "maps/Test/global.cfg", &content, &original).unwrap();
            assert_eq!(std::fs::read(&saved).unwrap(), first);
        }
        let tile = omsi_map::Tile::load(&saved).unwrap();
        assert_eq!(tile.objects[0].pos, [22.0, 43.0, 1.75]); assert_eq!(tile.objects[0].rot[0], 20.0);
        world.object_edits.lock().insert(7, ObjectEdit::default()); // Backspace restores this session's source.
        editor.save(&world, "maps/Test/global.cfg", &content, &original).unwrap();
        let restored = omsi_map::Tile::load(&saved).unwrap();
        assert_eq!(restored.objects[0].pos, [20.0, 40.0, 1.25]); assert_eq!(restored.objects[0].rot[0], 15.0);
        assert_eq!(std::fs::read_to_string(map.join("tile_0_0.map")).unwrap(), source);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn copied_tree_keeps_labels_and_saves_relative_height_on_another_tile_then_deletes() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("omsi-object-copy-{}-{stamp}",std::process::id()));
        let original=dir.join("original");let content=dir.join("mod");let map=original.join("maps/Test");
        let scenery=original.join("Sceneryobjects");std::fs::create_dir_all(&map).unwrap();std::fs::create_dir_all(&scenery).unwrap();
        std::fs::write(scenery.join("tree.sco"),"[tree]\nleaf.png\n6\n10\n0.3\n0.5\n").unwrap();
        std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n").unwrap();
        let source="[version]\n14\n[terrain]\n[object]\n0\nSceneryobjects\\tree.sco\n7\n20\n40\n1.25\n15\n0\n0\n4\nleaf2.png\n8\n0.4\n\n";
        std::fs::write(map.join("tile_0_0.map"),source).unwrap();
        std::fs::write(map.join("tile_1_0.map"),"[version]\n14\n[terrain]\n").unwrap();
        let world=World::open(&original,&map.join("global.cfg"),20000101).unwrap();
        let mut terrain=omsi_map::Terrain::flat();terrain.heights.fill(40.0);
        for key in [(0,0),(1,0)] {world.terrains.write().insert(key,std::sync::Arc::new(terrain.clone()));}
        let strings=vec!["leaf2.png".into(),"8".into(),"0.4".into()];
        world.edit_objects.lock().insert(7,crate::scene::EditObject {tile:(0,0),pos:DVec3::new(20.0,40.0,41.25),
            xf:Mat4::from_rotation_z(-15.0_f32.to_radians()),key:7,instances:Vec::new(),sco:scenery.join("tree.sco"),strings});
        let mut editor=Editor::default();editor.selected=Some(7);
        let clip=editor.capture_object(&world).unwrap();assert_eq!(clip.height_offset,1.25);
        assert_eq!(clip.strings,vec!["leaf2.png","8","0.4",""]);
        editor.clipboard=Some(clip.clone());editor.start_object(clip.clone());
        world.edit_objects.lock().clear(); // The clipboard survives source unloading/deletion.
        assert_eq!(editor.object_stamp.as_ref().unwrap().strings,clip.strings);
        let mut placed=added_object(501);placed.tile=(1,0);placed.sco=clip.sco;placed.strings=clip.strings;
        placed.base=DVec3::new(320.0,40.0,41.25);placed.ground_height=40.0;placed.base_heading=clip.heading;editor.added.push(placed);
        editor.save(&world,"maps/Test/global.cfg",&content,&original).unwrap();
        let saved=content.join("maps/Test/tile_1_0.map");let tile=omsi_map::Tile::load(&saved).unwrap();
        assert_eq!(tile.objects.len(),1);assert_eq!(tile.objects[0].pos,[20.0,40.0,1.25]);
        assert_eq!(tile.objects[0].flag,4);assert_eq!(tile.objects[0].extra,vec!["leaf2.png","8","0.4"]);
        editor.save(&world,"maps/Test/global.cfg",&content,&original).unwrap();
        assert_eq!(omsi_map::Tile::load(&saved).unwrap().objects.len(),1);
        editor.added[0].deleted=true;editor.save(&world,"maps/Test/global.cfg",&content,&original).unwrap();
        assert!(omsi_map::Tile::load(&saved).unwrap().objects.is_empty());
        assert_eq!(std::fs::read_to_string(map.join("tile_0_0.map")).unwrap(),source);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn object_angles_keep_spline_bindings_and_native_saved_pose() {
        use winit::keyboard::KeyCode as K;
        assert!(matches!(action_for_mode(K::Home,false,false,0.0,false),Some(Action::Tilt(0,0.5))));
        assert!(matches!(action_for_mode(K::PageDown,true,false,0.0,false),Some(Action::Tilt(1,v)) if v == -0.05));
        assert!(matches!(action_for_mode(K::Home,false,false,0.0,true),Some(Action::Grade(0.5))));
        assert!(matches!(action_for_mode(K::Comma,false,false,0.0,true),Some(Action::Curvature(v)) if v == -0.001));
        assert!(action_for_mode(K::Comma,false,false,0.0,false).is_none());
        assert!(action_for_mode(K::Period,false,false,0.0,false).is_none());
        assert!(matches!(action_for_mode(K::Delete,false,false,0.0,false),Some(Action::Delete)));
        assert!(matches!(action_for_mode(K::Backspace,false,false,0.0,false),Some(Action::Undo)));
        let original=[2.0,-1.5];
        let tilt=crate::object_angles::adjusted(original,0,0.05).unwrap();
        let mut object=PlacedRecord{id:708,file:"Sceneryobjects\\openOMSI_Editor\\Junctions\\test\\junction.sco".into(),pos:DVec3::new(10.0,20.0,3.0),heading:35.0,tilt,strings:vec!["Name".into()],deleted:false};
        let other="[object]\n0\nother.sco\n709\n1\n2\n3\n4\n5\n6\n0\n";
        let saved=rewrite_placed(other,std::slice::from_ref(&object)).unwrap().0;
        assert!(saved.starts_with(other));
        let parsed=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&saved));
        let placed=parsed.objects.iter().find(|o|o.id==708).unwrap();
        assert_eq!(placed.rot,[35.0,2.05,-1.5]);
        object.heading+=5.0;object.pos.x+=0.5;
        let moved=rewrite_placed(&saved,std::slice::from_ref(&object)).unwrap().0;
        let parsed=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&moved));
        assert_eq!(parsed.objects.iter().find(|o|o.id==708).unwrap().rot,[40.0,2.05,-1.5]);
        object.tilt=original;
        let reset=rewrite_placed(&moved,std::slice::from_ref(&object)).unwrap().0;
        let parsed=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&reset));
        assert_eq!(parsed.objects.iter().find(|o|o.id==708).unwrap().rot,[40.0,2.0,-1.5]);
        assert_eq!(rewrite_placed(&reset,std::slice::from_ref(&object)).unwrap().0,reset);
    }

    #[test]
    fn labelled_object_rewrite_is_idempotent_preserves_tilt_and_can_remove_it() {
        let mut object=PlacedRecord {id:502,file:"Sceneryobjects\\sign.sco".into(),pos:DVec3::new(2.0,3.0,0.5),heading:30.0,
            tilt:[12.0,-8.0],strings:vec!["Hauptstraße".into(),"[object]".into(),String::new()],deleted:false};
        let base="[version]\r\n14\r\n[terrain]\r\n";
        let first=rewrite_placed(base,std::slice::from_ref(&object)).unwrap().0;
        assert_eq!(rewrite_placed(&first,std::slice::from_ref(&object)).unwrap().0,first);
        let parsed=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&first));
        assert_eq!(parsed.objects[0].rot,[30.0,12.0,-8.0]);
        assert_eq!(parsed.objects[0].extra,vec!["Hauptstraße","[object]"]);
        object.deleted=true;
        let removed=rewrite_placed(&first,&[object]).unwrap().0;
        assert!(omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&removed)).objects.is_empty());
    }

    #[test]
    fn pending_asset_placement_keeps_all_camera_directions_available() {
        use winit::keyboard::KeyCode as K;
        let mut editor = Editor::default();
        editor.placing_asset = Some(crate::asset_catalog::Asset {
            kind: crate::asset_catalog::Kind::Object,category:crate::asset_catalog::Category::Other,groups:String::new(),file: "Sceneryobjects/test.sco".into(),
            name: "test".into(), path: PathBuf::from("Sceneryobjects/test.sco"),
        });
        let keys = hashbrown::HashSet::from([K::KeyW, K::KeyA, K::KeyS, K::KeyD, K::KeyQ, K::KeyE]);
        for code in keys.iter().copied() { assert!(camera_key_pressed(&keys, Some(&editor), code)); }
        let saving=hashbrown::HashSet::from([K::ControlLeft,K::KeyS]);
        assert!(!camera_key_pressed(&saving,Some(&editor),K::KeyS));
        assert!(one_shot_key(K::KeyC,false,true));assert!(one_shot_key(K::KeyV,false,true));
    }

    #[test]
    fn inserted_junction_saves_once_moves_and_can_be_removed() {
        let mut junction = PlacedRecord { id: 500, file: "Sceneryobjects\\junction.sco".into(),
            pos: DVec3::new(10.0, 20.0, 3.0), heading: 45.0,tilt:[0.0;2],strings:Vec::new(),deleted: false };
        let base = "[version]\r\n14\r\n\r\n[terrain]\r\n";
        let (first, _) = rewrite_placed(base, &[junction]).unwrap();
        junction = PlacedRecord { id: 500, file: "Sceneryobjects\\junction.sco".into(),
            pos: DVec3::new(10.0, 20.0, 3.0), heading: 45.0,tilt:[0.0;2],strings:Vec::new(),deleted: false };
        assert_eq!(rewrite_placed(&first, &[junction]).unwrap().0, first);
        junction = PlacedRecord { id: 500, file: "Sceneryobjects\\junction.sco".into(),
            pos: DVec3::new(12.0, 20.0, 3.0), heading: 90.0,tilt:[0.0;2],strings:Vec::new(),deleted: false };
        let moved = rewrite_placed(&first, &[junction]).unwrap().0;
        let tile = omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map", &moved));
        assert_eq!(tile.objects.len(), 1);
        assert_eq!(tile.objects[0].pos, [12.0, 20.0, 3.0]);
        assert_eq!(tile.objects[0].rot[0], 90.0);
        junction = PlacedRecord { id: 500, file: "Sceneryobjects\\junction.sco".into(),
            pos: DVec3::new(12.0, 20.0, 3.0), heading: 90.0,tilt:[0.0;2],strings:Vec::new(),deleted: true };
        let deleted = rewrite_placed(&moved, &[junction]).unwrap().0;
        assert!(!deleted.contains("[object]"));
        assert!(deleted.contains("[terrain]\r\n"));
    }

    const TILE: &str = "[version]\r\n4\r\n\r\n[object]\r\n0\r\nSceneryobjects\\a.sco\r\n7\r\n5\r\n6\r\n0.25\r\n90\r\n0\r\n0\r\n0\r\n\r\nObject Nr. 1\r\n[object]\r\n0\r\nSceneryobjects\\b.sco\r\n8\r\n1\r\n2\r\n0\r\n0\r\n0\r\n0\r\n2\r\nHalt\r\nx\r\n\r\n[spline]\r\n0\r\n";

    #[test]
    fn a_moved_object_changes_its_lines_and_a_deleted_one_goes() {
        let mut edits = HashMap::new();
        edits.insert(7, ObjectEdit { moved: DVec3::new(1.5, -1.0, 0.0), turned: 12.5, deleted: false });
        edits.insert(8, ObjectEdit { deleted: true, ..Default::default() });
        let (out, n) = rewrite_tile(TILE, &edits);
        assert_eq!(n, 2);
        assert!(out.contains("[object]\r\n0\r\nSceneryobjects\\a.sco\r\n7\r\n6.5\r\n5\r\n0.25\r\n102.5\r\n0\r\n"), "{out:?}");
        assert!(!out.contains("b.sco") && !out.contains("Halt"), "{out:?}");
        assert!(out.ends_with("[spline]\r\n0\r\n"));
        // nothing to do: the same text
        assert_eq!(rewrite_tile(TILE, &HashMap::new()), (TILE.to_string(), 0));
    }

    #[test]
    fn a_utf16_tile_is_written_back_as_utf16() {
        let bytes = encode(TILE, Encoding::Utf16Le);
        assert_eq!(&bytes[..4], &[0xFF, 0xFE, b'[', 0]);
        let (text, enc) = decode(&bytes);
        assert_eq!((text.as_str(), enc), (TILE, Encoding::Utf16Le));
    }
    #[test]
    fn sign_labels_keep_slots_endings_keywords_and_other_records() {
        let source = "[version]\r\n14\r\n[object]\r\n0\r\nSceneryobjects\\sign.sco\r\n7\r\n1\r\n2\r\n3\r\n45\r\n0\r\n0\r\n2\r\nAlt\r\n\r\n[unknown]\r\nKeep\r\n";
        let labels = HashMap::from([(7,vec!["München".into(),"[object]".into(),String::new()])]);
        let (updated,count) = rewrite_labels(source,&labels).unwrap(); assert_eq!(count,1);
        assert!(updated.contains("3\r\nMünchen\r\n[object]\r\n\r\n[unknown]\r\nKeep\r\n"));
        assert_eq!(rewrite_labels(&updated,&labels).unwrap().0,updated);
        let (moved,_) = rewrite_tile(&updated,&HashMap::from([(7,ObjectEdit {moved:DVec3::X,..Default::default()})]));
        assert!(moved.contains("7\r\n2\r\n2\r\n3\r\n45\r\n"));
        assert!(moved.contains("München\r\n[object]\r\n\r\n[unknown]"));
        let (deleted,_) = rewrite_tile(&updated,&HashMap::from([(7,ObjectEdit {deleted:true,..Default::default()})]));
        assert_eq!(deleted,"[version]\r\n14\r\n[unknown]\r\nKeep\r\n");
        assert!(rewrite_labels(source,&HashMap::from([(99,vec!["Unbekannt".into()])])).is_err());
        let unicode = "Ort: Łódź"; let encoded = encode(unicode,Encoding::Latin1);
        assert_eq!(decode(&encoded),(unicode.into(),Encoding::Utf16Le));
    }
    #[test]
    fn editing_a_maps_sign_text_saves_without_moving_it_and_camera_is_blocked_in_dialog() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("omsi-sign-save-{}-{stamp}",std::process::id()));
        let original=dir.join("original");let content=dir.join("mod");let map=original.join("maps/Test");
        std::fs::create_dir_all(&map).unwrap();
        std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n").unwrap();
        let source="[version]\n14\n[terrain]\n[object]\n0\nSceneryobjects\\sign.sco\n7\n1\n2\n3\n45\n0\n0\n1\nAlt\n";
        std::fs::write(map.join("tile_0_0.map"),source).unwrap();
        let world=World::open(&original,&map.join("global.cfg"),20000101).unwrap();
        world.object_text_edits.lock().insert(7,vec!["Neu: München".into(),String::new()]);
        let mut editor=Editor::default();editor.tiles.insert(7,(0,0));
        editor.save(&world,"maps/Test/global.cfg",&content,&original).unwrap();
        let out=std::fs::read_to_string(content.join("maps/Test/tile_0_0.map")).unwrap();
        assert!(out.contains("7\n1\n2\n3\n45\n0\n0\n2\nNeu: München\n\n"));
        assert_eq!(std::fs::read_to_string(map.join("tile_0_0.map")).unwrap(),source);
        editor.text_window=Some(crate::object_text::Window::new(crate::object_text::Target::Map(7),"Schild".into(),
            vec![(0,"Ort".into())],vec!["München".into()]).unwrap());
        use winit::keyboard::KeyCode as K;
        let keys=HashSet::from([K::KeyW,K::KeyA,K::KeyS,K::KeyD,K::KeyQ,K::KeyE]);
        for key in keys.iter().copied() {assert!(!camera_key_pressed(&keys,Some(&editor),key));}
        let _=std::fs::remove_dir_all(dir);
    }

    #[test]
    fn spline_edits_keep_utf16_and_unknown_tile_content() {
        let text = "[version]\r\n14\r\n\r\n[terrain]\r\n\r\n[spline]\r\n0\r\nSplines\\road.sli\r\n99\r\n0\r\n0\r\n10\r\n5\r\n20\r\n0\r\n30\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n\r\n[unknown]\r\nGrüße\r\n";
        let bytes = encode(text, Encoding::Utf16Le);
        let (decoded, enc) = decode(&bytes);
        let mut tile = omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("tile.map", &decoded));
        let mut s = tile.splines.remove(0);
        s.pos[2] += 1.25;
        s.is_h = true;
        s.delta_h = Some(-1.0);
        let (new, _) = crate::spline_editor::rewrite(&decoded, &HashMap::from([(99, s)]), &[]).unwrap();
        let result = encode(&new, enc);
        assert_eq!(&result[..2], &[0xff, 0xfe]);
        let (again, _) = decode(&result);
        assert!(again.contains("[unknown]\r\nGrüße\r\n"));
        let parsed = omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("tile.map", &again));
        assert_eq!(parsed.splines[0].pos, [10.0, 20.0, 6.25]);
        assert_eq!(parsed.splines[0].delta_h, Some(-1.0));
    }
}

#[cfg(test)]
mod copy_tests {
    #[test]
    fn a_copy_follows_its_template_with_its_own_id() {
        let text = "[object]\r\n0\r\nSceneryobjects\\A\\post.sco\r\n17\r\n10\r\n20\r\n0\r\n90\r\n\r\n[object]\r\n0\r\nx.sco\r\n18\r\n1\r\n1\r\n0\r\n0\r\n";
        let (out, n) = super::add_copies(text, &[super::NewRecord { template: 17, id: 99, file: "lamp.sco".into(), moved: glam::DVec3::new(2.0, 0.0, 0.0), turned: 10.0 }]);
        assert_eq!(n, 1);
        assert!(out.contains("Sceneryobjects\\A\\lamp.sco\r\n99\r\n12\r\n20\r\n0\r\n100\r\n"), "{out}");
        assert!(out.contains("[object]\r\n0\r\nx.sco\r\n18"));
    }
}

#[cfg(test)]
mod junction_persistence_tests {
    use super::*;
    #[test]fn replacement_keeps_id_pose_and_other_instances_and_is_repeatable(){
        let text="[version]\n14\n[object]\n0\nold/junction.sco\n99\n10\n20\n3\n37\n0\n0\n0\n[object]\n0\nold/junction.sco\n100\n40\n50\n3\n0\n0\n0\n0\n";
        let records=[PlacedRecord{id:99,file:"new/junction.sco".into(),pos:DVec3::new(10.0,20.0,3.0),heading:37.0,tilt:[0.0;2],strings:Vec::new(),deleted:false}];
        let(updated,_)=rewrite_placed(text,&records).unwrap();
        let(repeated,_)=rewrite_placed(&updated,&records).unwrap();assert_eq!(updated,repeated);
        let tile=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&updated));
        assert_eq!(tile.objects.len(),2);let a=tile.objects.iter().find(|o|o.id==99).unwrap();
        assert_eq!(a.file,"new/junction.sco");assert_eq!(a.pos,[10.0,20.0,3.0]);assert_eq!(a.rot,[37.0,0.0,0.0]);
        assert_eq!(tile.objects.iter().find(|o|o.id==100).unwrap().file,"old/junction.sco");
        // The loaded-object replacement path first hides/removes the old record.
        let edits=HashMap::from([(99,ObjectEdit{deleted:true,..Default::default()})]);let(hidden,_)=rewrite_tile(text,&edits);
        let(saved,_)=rewrite_placed(&hidden,&records).unwrap();let tile=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&saved));
        assert_eq!(tile.objects.iter().filter(|o|o.id==99).count(),1);assert_eq!(tile.objects.len(),2);
    }
}

#[cfg(test)]
mod portable_save_tests {
    use super::save_copy;

    #[test]
    fn repeated_save_in_unicode_directory_keeps_previous_bytes() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("editor Straße test-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("tile_-12_13.map");
        save_copy(&file, b"original").unwrap();
        save_copy(&file, b"edited").unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"edited");
        let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
        let backups: Vec<_> = entries.iter().filter(|p| p.file_name().unwrap().to_string_lossy().contains("before-editor")).collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read(backups[0]).unwrap(), b"original");
        assert!(!entries.iter().any(|p| p.extension().is_some_and(|e| e == "tmp")));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn upstream_build_keeps_its_update_policy() {
        assert_eq!(crate::spline_editor::OFFICIAL_UPDATES_DISABLED, cfg!(feature = "standalone-editor"));
    }
}
