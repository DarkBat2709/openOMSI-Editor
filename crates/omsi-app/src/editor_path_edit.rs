//! Per-instance path edit state and interaction.
pub use omsi_scene::editor_paths::*;
use glam::DVec3;
use omsi_sim::traffic::Lane;
#[cfg(test)]
use omsi_sim::traffic::LaneKey;
use std::collections::HashMap;
/// The simulation retains object lanes until reload; hide deleted owners in the editor.
/// Spline paths have a separate identity and must not be removed by an object deletion.
pub fn owner_is_deleted(lane: &Lane, deleted_objects: &std::collections::HashSet<i64>) -> bool {
    lane.source == 2 && lane.key.is_some_and(|key| deleted_objects.contains(&key.id))
}

#[derive(Default)]
pub struct Edits {
    pub changes: HashMap<Id, Change>,
    baseline: HashMap<Id, Change>,
    undo: Vec<(Id, Option<Change>, Change)>,
    redo: Vec<(Id, Option<Change>, Change)>,
}
impl Edits {
    pub fn current(&self, l: &Lane) -> Change {
        Id::of(l)
            .and_then(|id| self.changes.get(&id).copied())
            .unwrap_or(Change {
                deleted: false,
                shape: l.editor_path_shape, ends:l.editor_path_ends, floor:l.editor_path_floor,
            })
    }
    pub fn change(&mut self, l: &Lane, c: Change) {
        let Some(id) = Id::of(l) else {
            return;
        };
        if self.current(l) == c {
            return;
        }
        self.baseline.entry(id).or_insert(Change {
            deleted: false,
            shape: l.editor_path_shape, ends:l.editor_path_ends, floor:l.editor_path_floor,
        });
        let before = self.changes.insert(id, c);
        self.undo.push((id, before, c));
        self.redo.clear();
    }
    pub fn undo(&mut self) {
        if let Some(h) = self.undo.pop() {
            if let Some(c) = h.1 {
                self.changes.insert(h.0, c);
            } else {
                self.changes.insert(h.0, self.baseline[&h.0]);
            }
            self.redo.push(h);
        }
    }
    pub fn redo(&mut self) {
        if let Some(h) = self.redo.pop() {
            self.changes.insert(h.0, h.2);
            self.undo.push(h);
        }
    }
    pub fn drag(&mut self, l: &Lane, c: Change) {
        if let Some(id) = Id::of(l) {
            self.baseline.entry(id).or_insert(Change {
                deleted: false,
                shape: l.editor_path_shape, ends:l.editor_path_ends, floor:l.editor_path_floor,
            });
            self.changes.insert(id, c);
        }
    }
    pub fn commit_drag(&mut self, l: &Lane, before: Change, after: Change) {
        if let Some(id) = Id::of(l) {
            if before != after {
                self.undo.push((id, Some(before), after));
                self.redo.clear();
            }
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn preview<'a>(&self, l: &'a Lane) -> Option<std::borrow::Cow<'a, Lane>> {
        let c = self.current(l);
        if c.deleted {
            return None;
        }
        if c.shape == l.editor_path_shape && c.ends==l.editor_path_ends && c.floor==l.editor_path_floor {
            return Some(std::borrow::Cow::Borrowed(l));
        }
        let mut l = l.clone();
        l.editor_path_ends=c.ends;l.editor_path_floor=c.floor;apply_shape(&mut l, c.shape);
        Some(std::borrow::Cow::Owned(l))
    }
}
#[derive(Clone, Copy)]
pub struct PathDrag {pub index:usize,pub point:DVec3,pub normal:DVec3,pub offset:DVec3,pub before:Change}
impl PathDrag {
    pub fn target(&self,origin:DVec3,ray:DVec3)->Option<DVec3> {
        let divisor=ray.dot(self.normal);if divisor.abs()<1e-6{return None;}
        let t=(self.point-origin).dot(self.normal)/divisor;
        (t>0.0&&t<2000.0).then_some(origin+ray*t+self.offset-DVec3::Z*0.4)
    }
}
#[derive(Default)]
pub struct Manager {
    pub visible: Vec<Lane>,
    pub selected: Option<Lane>,
    pub candidates: Vec<Lane>,
    pub candidate: usize,
    pub editing: bool,
    pub deleted: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub drag: Option<PathDrag>,
}
impl Manager {
    pub fn pick(&mut self, origin: DVec3, ray: DVec3) {
        let mut hits: Vec<_> = self
            .visible
            .iter()
            .filter_map(|l| {
                let distance = l
                    .points
                    .windows(2)
                    .filter_map(|p| {
                        ray_segment(origin, ray, p[0] + DVec3::Z * 0.25, p[1] + DVec3::Z * 0.25)
                    })
                    .min_by(f64::total_cmp)?;
                Some((distance, l.clone()))
            })
            .collect();
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.candidates = hits.into_iter().take(12).map(|(_, l)| l).collect();
        self.candidate = 0;
        self.selected = self.candidates.first().cloned();
        self.editing = self.selected.as_ref().is_some_and(|l| l.points.len() >= 2);
    }
    pub fn next(&mut self) {
        if !self.candidates.is_empty() {
            self.candidate = (self.candidate + 1) % self.candidates.len();
            self.selected = Some(self.candidates[self.candidate].clone());
            self.editing = false;
        }
    }
    pub fn control(l: &Lane, index: usize) -> (DVec3, f64) {
        let at = (l.points.len().saturating_sub(1)) as f64 * (index + 1) as f64 / 3.0;
        let i = (at.floor() as usize).min(l.points.len().saturating_sub(2));
        let t = at - i as f64;
        let a = l.points[i];
        let b = l.points[i + 1];
        let d = b - a;
        (a.lerp(b, t), d.x.atan2(d.y).to_degrees())
    }
    pub fn handles(l: &Lane) -> [(usize, DVec3); 4] {
        [(0, Self::control(l, 0).0), (1, Self::control(l, 1).0),(2,l.start()),(3,l.end())]
    }
}
fn ray_segment(o: DVec3, d: DVec3, a: DVec3, b: DVec3) -> Option<f64> {
    let v = b - a;
    let w = o - a;
    let vv = v.length_squared();
    if vv < 1e-12 {
        return None;
    }
    let dv = d.dot(v);
    let denom = vv - dv * dv;
    let u = if denom > 1e-10 {
        ((w.dot(v) - d.dot(w) * dv) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let p = a + v * u;
    let t = (p - o).dot(d);
    if t <= 0.0 || t > 1000.0 {
        return None;
    }
    let dist = (p - o - d * t).length();
    (dist < (t * 0.006).clamp(0.35, 1.5)).then_some(dist)
}

/// Replace only our own rules on the selected record. Preserve every unrelated byte,
/// path index, traffic rule and unknown section, and reject missing/duplicate records.
pub fn rewrite(text: &str, tile: (i32, i32), edits: &Edits) -> Result<(String, usize), String> {
    let wanted: Vec<_> = edits
        .changes
        .iter()
        .filter(|(id, _)| id.key.tile == tile)
        .collect();
    if wanted.is_empty() {
        return Ok((text.into(), 0));
    }
    let cfg = omsi_cfg::CfgFile::from_str("paths.map", text);
    let parsed = omsi_map::Tile::parse(&cfg);
    if parsed.version < 14 {
        return Err("AI path editing requires a version 14 tile".into());
    }
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::new();
    let mut owner = None;
    let mut found = HashMap::new();
    let mut i = 0;
    let record = |s: &str| match s.trim().to_ascii_lowercase().as_str() {
        "[object]" | "[attachobj]" => Some(2),
        "[spline]" | "[spline_h]" => Some(1),
        _ => None,
    };
    let append = |out: &mut String, owner: Option<(u32, i64)>| {
        for (id, c) in &wanted {
            if owner != Some((id.source, id.key.id)) {
                continue;
            }
            let mut emit = |name: String, value: f64| {
                out.push_str(&format!(
                    "[rule]{eol}{}{eol}{name}{eol}{value}{eol}0{eol}",
                    id.key.path
                ));
            };
            emit(
                format!("{PREFIX}deleted_{}", suffix(id.reversed)),
                if c.deleted { 1.0 } else { 0.0 },
            );
            if let Some(floor)=c.floor {for (i,z) in floor.into_iter().enumerate(){emit(format!("{PREFIX}floor_{i}_{}",suffix(id.reversed)),z);}}
            for (n,name) in ["start","end"].iter().enumerate() {for (axis,value) in ["x","y","z"].iter().zip(c.ends[n]) {emit(format!("{PREFIX}{name}_{axis}_{}",suffix(id.reversed)),value);}}
            for n in 0..2 {
                for (axis, value) in ["x", "y", "z"].iter().zip(c.shape[n]) {
                    emit(
                        format!("{PREFIX}{}_{axis}_{}", n + 1, suffix(id.reversed)),
                        value,
                    );
                }
            }
        }
    };
    while i < lines.len() {
        if let Some(source) = record(lines[i]) {
            append(&mut out, owner);
            owner = lines
                .get(i + 3)
                .and_then(|s| s.trim().parse::<i64>().ok())
                .map(|id| (source, id));
            if wanted
                .iter()
                .any(|(id, _)| owner == Some((id.source, id.key.id)))
            {
                *found.entry(owner.unwrap()).or_insert(0) += 1;
            }
        }
        // A rule belongs to the latest object/spline until a different element starts.
        if lines[i].trim().eq_ignore_ascii_case("[rule]") && i + 4 < lines.len() {
            let path = lines[i + 1].trim().parse::<u16>().ok();
            let name = lines[i + 2].trim();
            if wanted.iter().any(|(id, _)| {
                owner == Some((id.source, id.key.id))
                    && path == Some(id.key.path)
                    && name.starts_with(PREFIX)
                    && name.ends_with(suffix(id.reversed))
            }) {
                i += 5;
                continue;
            }
        }
        // Terrain and attachment rows end a record's rules.
        let h = lines[i].trim().to_ascii_lowercase();
        if [
            "[terrain]",
            "[water]",
            "[splineattachement]",
            "[splineattachement_repeater]",
        ]
        .contains(&h.as_str())
        {
            append(&mut out, owner);
            owner = None;
        }
        out.push_str(lines[i]);
        i += 1;
    }
    if !out.ends_with('\n') {
        out.push_str(eol);
    }
    append(&mut out, owner);
    for (id, _) in &wanted {
        if found.get(&(id.source, id.key.id)) != Some(&1) {
            return Err(format!(
                "AI path owner {} missing or duplicated; nothing saved",
                id.key.id
            ));
        }
    }
    Ok((out, wanted.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lane(id: i64, source: u32, reverse: bool) -> Lane {
        let mut l = omsi_sim::traffic::LaneBuilder::arc(
            DVec3::ZERO,
            0.0,
            30.0,
            0.0,
            0.0,
            omsi_sim::traffic::LaneKind::Street,
            3.0,
        );
        l.key = Some(LaneKey {
            tile: (0, 0),
            id,
            path: 0,
        });
        l.source = source;
        l.reversed = reverse;
        l
    }
    #[test]
    fn drag_from_horizontal_side_view_does_not_need_a_ground_plane_intersection() {
        let p=DVec3::new(0.0,10.0,2.4);
        let drag=PathDrag {index:0,point:p,normal:DVec3::Y,offset:DVec3::ZERO,before:Change::default()};
        assert_eq!(drag.target(DVec3::new(0.0,0.0,2.4),DVec3::Y).unwrap(),DVec3::new(0.0,10.0,2.0));
        let moved=drag.target(DVec3::new(0.0,0.0,2.4),DVec3::new(0.1,1.0,0.0).normalize()).unwrap();
        assert!((moved.x-1.0).abs()<1e-6);assert_eq!(moved.z,2.0);
    }
    #[test]
    fn short_path_can_be_bent_and_follow_next_road_section() {
        let a = omsi_geometry::SplineCurve::from_map(
            &omsi_map::MapSpline {
                length: 4.9,
                grad_start: 10.0,
                grad_end: 10.0,
                ..Default::default()
            },
            glam::DVec2::ZERO,
        );
        let b = omsi_geometry::SplineCurve::from_map(
            &omsi_map::MapSpline {
                length: 40.0,
                pos: [0.0, 4.9, 0.49],
                grad_start: 10.0,
                grad_end: 10.0,
                ..Default::default()
            },
            glam::DVec2::ZERO,
        );
        let points = vec![
            a.profile_point(0.0, 0.0, 0.1),
            a.profile_point(4.9, 0.0, 0.1),
        ];
        let mut l = omsi_sim::traffic::LaneBuilder::polyline(
            points.clone(),
            omsi_sim::traffic::LaneKind::Street,
            3.0,
        );
        l.editor_path_surface = Some(omsi_sim::traffic::EditorPathSurface::Spline(a));
        attach_roads(std::slice::from_mut(&mut l), &[a, b],true);
        apply_shape(&mut l, [[0.0, 22.0, 0.0]; 2]);
        assert!(l.points.len() > 15);
        assert!(l.points.iter().any(|p| p.y > 20.0));
        for p in &l.points {
            assert!((p.z - p.y * 0.1 - 0.1).abs() < 1e-5, "{p:?}");
        }
        assert_eq!(l.start(), points[0]);
        assert_eq!(l.end(), points[1]);
        let mut m = Manager {
            visible: vec![l.clone()],
            ..Default::default()
        };
        let h = Manager::handles(&l)[0].1;
        m.pick(h + DVec3::Z * 10.0, -DVec3::Z);
        assert!(m.editing);
    }
    #[test]
    #[ignore = "requires user map paths in OPENOMSI_PATH_TEST_ROOT and OPENOMSI_PATH_TEST_TILE"]
    fn user_map_shifted_short_path_follows_visible_mesh() {
        let root = std::path::PathBuf::from(std::env::var("OPENOMSI_PATH_TEST_ROOT").unwrap());
        let tile = omsi_map::Tile::load(&std::path::PathBuf::from(
            std::env::var("OPENOMSI_PATH_TEST_TILE").unwrap(),
        ))
        .unwrap();
        let origin = glam::DVec2::new(-18.0, 10.0) * omsi_map::tile_size();
        let mut roads = Vec::new();
        let mut meshes = Vec::new();
        let mut selected = None;
        for sp in &tile.splines {
            if sp.deleted {
                continue;
            }
            let Ok(def) = omsi_scenery::Spline::load(&omsi_cfg::resolve_path(&root, &sp.file))
            else {
                continue;
            };
            if def.only_editor {
                continue;
            }
            let curve = omsi_geometry::SplineCurve::from_map(sp, origin).with_sli(&def);
            roads.push(curve);
            meshes.push(omsi_geometry::build_spline_mesh(
                &def,
                &curve,
                sp.mirror,
                DVec3::ZERO,
            ));
            if sp.id == 9859315 {
                let clean = omsi_map::MapSpline {
                    rules: vec![],
                    ..sp.clone()
                };
                let world = crate::scene::World::open(
                    &root,
                    &std::path::PathBuf::from(std::env::var("OPENOMSI_PATH_TEST_GLOBAL").unwrap()),
                    20000101,
                )
                .unwrap();
                selected = world
                    .editor_path_lanes(&def, &clean, &curve, (-18, 10))
                    .into_iter()
                    .find(|l| l.kind == omsi_sim::traffic::LaneKind::Street && l.reversed);
            }
        }
        let mut l = selected.expect("path 9859315 missing");
        attach_roads(std::slice::from_mut(&mut l), &roads,true);
        apply_shape(
            &mut l,
            [
                [6.564280383772964, 22.474305937937828, 0.0],
                [-8.004607650752178, 28.14674447354651, 0.0],
            ],
        );
        let mut checked = 0;
        let mut max_gap = 0.0_f64;
        for p in &l.points {
            let heights: Vec<_> = meshes
                .iter()
                .filter_map(|m| crate::scene::field_height(m, p.x as f32, p.y as f32))
                .collect();
            if let Some(h) = heights.iter().min_by(|a, b| {
                (**a as f64 - p.z)
                    .abs()
                    .total_cmp(&(**b as f64 - p.z).abs())
            }) {
                let gap = (p.z - *h as f64).abs();
                max_gap = max_gap.max(gap);
                checked += 1;
            }
        }
        println!(
            "Real map: {checked} samples above visible road meshes; max height gap {max_gap:.4} m"
        );
        assert!(checked > 10);
        assert!(max_gap < 0.35, "height gap {max_gap}");
    }
    #[test]
    fn moved_paths_follow_graded_banked_roads_in_both_directions() {
        use omsi_sim::traffic::EditorPathSurface;
        let road = omsi_map::MapSpline {
            length: 30.0,
            grad_start: 10.0,
            grad_end: 10.0,
            cant_start: 8.0,
            cant_end: 8.0,
            pos: [0.0, 0.0, 20.0],
            ..Default::default()
        };
        let curve = omsi_geometry::SplineCurve::from_map(&road, glam::DVec2::ZERO);
        for reversed in [false, true] {
            let mut points: Vec<_> = (0..=30)
                .map(|i| curve.profile_point(i as f64, 0.0, 0.1))
                .collect();
            if reversed {
                points.reverse();
            }
            let mut l = omsi_sim::traffic::LaneBuilder::polyline(
                points.clone(),
                omsi_sim::traffic::LaneKind::Street,
                3.0,
            );
            l.reversed = reversed;
            l.editor_path_surface = Some(EditorPathSurface::Spline(curve));
            let shape = [[2.0, 3.0, 0.0]; 2];
            apply_shape(&mut l, shape);
            for p in &l.points {
                let h = surface_height(l.editor_path_surface.as_ref().unwrap(), *p, 15.0).unwrap();
                assert!((p.z - h - 0.1).abs() < 1e-5, "{p:?} {h}");
            }
            assert_eq!(l.points.first(), points.first());
            assert_eq!(l.points.last(), points.last());
            let once = l.points.clone();
            apply_shape(&mut l, shape);
            assert_eq!(l.points, once);
            apply_shape(&mut l, [[0.0; 3]; 2]);
            assert_eq!(l.points, points);
        }
    }
    #[test]
    fn moved_object_paths_follow_tilt_and_crossing_field_without_terrain() {
        use omsi_sim::traffic::EditorPathSurface;
        let field = omsi_geometry::MeshData {
            positions: vec![
                glam::Vec3::new(-50.0, -50.0, -5.0),
                glam::Vec3::new(50.0, -50.0, 5.0),
                glam::Vec3::new(0.0, 100.0, 0.0),
            ],
            indices: vec![0, 1, 2],
            ..Default::default()
        };
        let surface = EditorPathSurface::Object {
            pos: DVec3::new(0.0, 0.0, 40.0),
            transform: glam::Mat4::from_rotation_x(0.1),
            field: Some(std::sync::Arc::new(field)),
        };
        let points: Vec<_> = (0..=30)
            .map(|i| {
                let mut p = DVec3::new(0.0, i as f64, 40.0);
                p.z = surface_height(&surface, p, 0.0).unwrap() + 0.1;
                p
            })
            .collect();
        let mut l = omsi_sim::traffic::LaneBuilder::polyline(
            points.clone(),
            omsi_sim::traffic::LaneKind::Street,
            3.0,
        );
        l.editor_path_surface = Some(surface);
        apply_shape(&mut l, [[3.0, 2.0, 0.0]; 2]);
        for p in &l.points {
            let h = surface_height(l.editor_path_surface.as_ref().unwrap(), *p, 0.0).unwrap();
            assert!((p.z - h - 0.1).abs() < 1e-4);
        }
        assert_eq!(l.points.first(), points.first());
        assert_eq!(l.points.last(), points.last());
    }
    #[test]
    fn deletion_is_per_instance_path_and_direction_and_undoable() {
        let a = lane(1, 2, false);
        let b = lane(2, 2, false);
        let backward = lane(1, 2, true);
        let mut e = Edits::default();
        e.change(
            &a,
            Change {
                deleted: true,
                ..Default::default()
            },
        );
        assert!(e.preview(&a).is_none());
        assert!(e.preview(&b).is_some());
        assert!(e.preview(&backward).is_some());
        e.undo();
        assert!(e.preview(&a).is_some());
        e.redo();
        assert!(e.preview(&a).is_none());
    }
    #[test]
    fn shape_preserves_endpoints_and_survives_rules_without_accumulation() {
        let mut a = lane(1, 1, false);
        let start = a.start();
        let end = a.end();
        let original = a.points.clone();
        let headings = a.headings.clone();
        let curvatures = a.curvature.clone();
        let shape = [[2.0, 0.0, 0.5], [-1.0, 0.0, 0.0]];
        apply_shape(&mut a, shape);
        assert_eq!(a.start(), start);
        assert_eq!(a.end(), end);
        assert_ne!(a.points, original);
        assert_eq!(a.headings.first(), headings.first());
        assert_eq!(a.headings.last(), headings.last());
        let first = a.points.clone();
        apply_shape(&mut a, shape);
        assert_eq!(a.points, first);
        apply_shape(&mut a, [[0.0; 3]; 2]);
        assert_eq!(a.points, original);
        assert_eq!(a.headings, headings);
        assert_eq!(a.curvature, curvatures);
    }
    #[test]
    fn saved_rules_keep_other_paths_instances_rules_and_crlf() {
        let text="[version]\r\n14\r\n[object]\r\n0\r\nx.sco\r\n1\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n[rule]\r\n0\r\nspeedlimit\r\n30\r\n0\r\n[object]\r\n0\r\nx.sco\r\n2\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n[terrain]\r\n";
        let mut e = Edits::default();
        e.change(
            &lane(1, 2, false),
            Change {
                deleted: true,
                shape: [[1.0, 0.0, 0.0], [0.0; 3]],
                ..Default::default()
            },
        );
        let out = rewrite(text, (0, 0), &e).unwrap().0;
        assert!(out.contains("speedlimit\r\n30"));
        assert_eq!(rewrite(&out, (0, 0), &e).unwrap().0, out);
        let tile = omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("tile.map", &out));
        assert_eq!(tile.objects.len(), 2);
        assert!(read(&tile.objects[0].rules, 0, false).deleted);
        assert!(!read(&tile.objects[0].rules, 0, true).deleted);
        assert!(tile.objects[1].rules.is_empty());
        let mut l = lane(1, 2, false);
        assert!(!from_rules(&mut l, &tile.objects[0].rules));
        e.undo();
        let restored = rewrite(&out, (0, 0), &e).unwrap().0;
        let tile = omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("tile.map", &restored));
        assert!(!read(&tile.objects[0].rules, 0, false).deleted);
    }
    #[test]
    fn ray_picks_between_samples_and_rejects_far_or_behind_paths() {
        assert!(ray_segment(
            DVec3::new(0.0, 5.0, 10.0),
            -DVec3::Z,
            DVec3::ZERO,
            DVec3::Y * 10.0
        )
        .is_some());
        assert!(ray_segment(
            DVec3::new(10.0, 5.0, 10.0),
            -DVec3::Z,
            DVec3::ZERO,
            DVec3::Y * 10.0
        )
        .is_none());
    }
    #[test]
    fn extended_end_and_road_heights_roundtrip_and_undo() {
        let l=lane(1,2,false);let end=l.end();let start=l.start();
        let mut e=Edits::default();let mut change=Change::default();
        change.ends[1]=[4.0,12.0,0.0];
        let mut heights=[0.0;65];for (i,z) in heights.iter_mut().enumerate(){*z=(i as f64/64.0*std::f64::consts::PI).sin()*2.0;}
        change.floor=Some(heights);e.change(&l,change);
        let view=e.preview(&l).unwrap();assert_eq!(view.start(),start);assert!((view.end()-end-DVec3::new(4.0,12.0,0.0)).length()<1e-6);
        assert!(view.points[view.points.len()/2].z>1.9);
        let text="[version]\n14\n[object]\n0\nx.sco\n1\n0\n0\n0\n0\n0\n0\n0\n[terrain]\n";
        let out=rewrite(text,(0,0),&e).unwrap().0;
        let tile=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("tile.map",&out));
        let saved=read(&tile.objects[0].rules,0,false);assert_eq!(saved,change);
        let mut loaded=lane(1,2,false);assert!(from_rules(&mut loaded,&tile.objects[0].rules));assert_eq!(loaded.points,view.points);
        assert_eq!(Manager::handles(&view).len(),4);
        e.undo();assert_eq!(e.preview(&l).unwrap().points,l.points);e.redo();assert_eq!(e.preview(&l).unwrap().points,loaded.points);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn instance_path_edits_save_reload_restore_and_leave_shared_assets_unchanged() {
        let dir = std::env::temp_dir().join(format!(
            "editor-path-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(dir.clone());
        let original = dir.join("original");
        let content = dir.join("content");
        let map = original.join("maps/PathTest");
        std::fs::create_dir_all(&map).unwrap();
        std::fs::create_dir_all(original.join("Sceneryobjects")).unwrap();
        std::fs::create_dir_all(original.join("Splines")).unwrap();
        let sco="[path]\n0\n0\n0\n0\n0\n30\n0\n0\n0\n3\n2\n0\n[path]\n3\n0\n0\n0\n0\n30\n0\n0\n0\n3\n0\n0\n";
        let sli = "[halfcantwidth]\n10\n[path]\n0\n0\n0\n3\n2\n[path]\n0\n3\n0\n3\n0\n";
        std::fs::write(original.join("Sceneryobjects/test.sco"), sco).unwrap();
        std::fs::write(original.join("Splines/test.sli"), sli).unwrap();
        let global = "[map]\n0\n0\ntile_0_0.map\n";
        std::fs::write(map.join("global.cfg"), global).unwrap();
        let objects="[version]\n14\n[object]\n0\nSceneryobjects/test.sco\n1\n10\n10\n0\n0\n0\n0\n0\n[object]\n0\nSceneryobjects/test.sco\n2\n50\n10\n0\n0\n0\n0\n0\n";
        let spline = omsi_map::MapSpline {
            id: 3,
            file: "Splines/test.sli".into(),
            pos: [100.0, 100.0, 20.0],
            grad_start: 10.0,
            grad_end: 10.0,
            cant_start: 8.0,
            cant_end: 8.0,
            length: 30.0,
            next_id: -1,
            ..Default::default()
        };
        let text = crate::spline_editor::rewrite(
            objects,
            &hashbrown::HashMap::from([(3, spline.clone())]),
            &[3],
        )
        .unwrap()
        .0;
        std::fs::write(map.join("tile_0_0.map"), &text).unwrap();
        let world =
            crate::scene::World::open(&original, &map.join("global.cfg"), 20000101).unwrap();
        let before = world.navigation_map().lanes;
        assert_eq!(before.len(), 9);
        let get = |lanes: &Vec<Lane>, id, source, path, rev| {
            lanes
                .iter()
                .find(|l| {
                    l.key.is_some_and(|k| k.id == id && k.path == path)
                        && l.source == source
                        && l.reversed == rev
                })
                .unwrap()
                .clone()
        };
        let mut ed = crate::editor::Editor::default();
        ed.path_edits.change(
            &get(&before, 1, 2, 0, false),
            Change {
                deleted: true,
                ..Default::default()
            },
        );
        ed.path_edits.change(
            &get(&before, 3, 1, 0, true),
            Change {
                deleted: true,
                ..Default::default()
            },
        );
        let shape = Change {
            shape: [[2.0, 3.0, 0.0], [-1.0, 2.0, 0.0]],
            ..Default::default()
        };
        ed.path_edits.change(&get(&before, 1, 2, 1, false), shape);
        ed.path_edits.change(&get(&before, 3, 1, 1, false), shape);
        ed.save(&world, "maps/PathTest/global.cfg", &content, &original)
            .unwrap();
        let saved = content.join("maps/PathTest/tile_0_0.map");
        let first = std::fs::read(&saved).unwrap();
        world.register_editor_tile((0, 0), 0, saved.clone());
        ed.save(&world, "maps/PathTest/global.cfg", &content, &original)
            .unwrap();
        assert_eq!(std::fs::read(&saved).unwrap(), first);
        std::fs::write(content.join("maps/PathTest/global.cfg"), global).unwrap();
        let reopened = crate::scene::World::open(
            &original,
            &content.join("maps/PathTest/global.cfg"),
            20000101,
        )
        .unwrap();
        let after = reopened.navigation_map().lanes;
        assert_eq!(after.len(), 7);
        assert!(!after
            .iter()
            .any(|l| l.key.is_some_and(|k| k.id == 1 && k.path == 0) && !l.reversed));
        assert!(!after
            .iter()
            .any(|l| l.key.is_some_and(|k| k.id == 3 && k.path == 0) && l.reversed));
        for (id, source, path) in [(1, 2, 1), (3, 1, 1)] {
            let a = get(&before, id, source, path, false);
            let b = get(&after, id, source, path, false);
            assert_eq!(a.start(), b.start());
            assert_eq!(a.end(), b.end());
            assert_ne!(a.points, b.points);
            if source == 1 {
                let curve = omsi_geometry::SplineCurve::from_map(&spline, glam::DVec2::ZERO);
                for p in &b.points {
                    let surface = omsi_sim::traffic::EditorPathSurface::Spline(curve);
                    let h = surface_height(&surface, *p, 15.0).unwrap();
                    assert!((p.z - h).abs() < 1e-4);
                }
                let preview=ed.path_edits.preview(&a).unwrap();assert_eq!(preview.points.len(),b.points.len());assert!(preview.points.iter().zip(&b.points).all(|(p,q)|p.distance(*q)<1e-9));
            }
        }
        for l in before.iter().filter(|l| l.key.unwrap().id == 2) {
            let same = get(&after, 2, 2, l.key.unwrap().path, l.reversed);
            assert_eq!(same.points, l.points);
        }
        for _ in 0..4 {
            ed.path_edits.undo();
        }
        ed.save(&world, "maps/PathTest/global.cfg", &content, &original)
            .unwrap();
        let restored = reopened.navigation_map().lanes;
        assert_eq!(restored.len(), 9);
        for l in &before {
            let k = l.key.unwrap();
            assert_eq!(
                get(&restored, k.id, l.source, k.path, l.reversed).points,
                l.points
            );
        }
        assert_eq!(
            std::fs::read_to_string(map.join("tile_0_0.map")).unwrap(),
            text
        );
        assert_eq!(
            std::fs::read_to_string(original.join("Sceneryobjects/test.sco")).unwrap(),
            sco
        );
        assert_eq!(
            std::fs::read_to_string(original.join("Splines/test.sli")).unwrap(),
            sli
        );
    }


}

#[cfg(test)]
mod deleted_owner_tests {
    use super::*;
    #[test]
    fn cached_object_paths_disappear_and_restore_without_hiding_independent_splines() {
        let mut object = omsi_sim::traffic::LaneBuilder::polyline(
            vec![DVec3::ZERO, DVec3::Y * 10.0], omsi_sim::traffic::LaneKind::Street, 3.0);
        object.source = 2;
        object.key = Some(LaneKey { tile: (-18, -2), id: 9861496, path: 937 });
        let mut spline = object.clone(); spline.source = 1;
        let mut other = object.clone(); other.key.as_mut().unwrap().id += 1;
        let cached = [object, spline, other];
        let mut deleted = std::collections::HashSet::from([9861496]);
        let visible: Vec<_> = cached.iter().filter(|lane| !owner_is_deleted(lane, &deleted)).collect();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].source, 1);
        assert_eq!(visible[1].key.unwrap().id, 9861497);
        deleted.clear();
        assert_eq!(cached.iter().filter(|lane| !owner_is_deleted(lane, &deleted)).count(), 3);
    }
}
