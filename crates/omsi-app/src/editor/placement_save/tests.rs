use super::*;
struct Fixture { dir: PathBuf, original: PathBuf, content: PathBuf, world: World }
impl Fixture {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("omsi-placement-tiles-{}-{stamp}", std::process::id()));
        let original = dir.join("original"); let content = dir.join("mod");
        let map = original.join("maps/Test"); let scenery = original.join("Sceneryobjects");
        std::fs::create_dir_all(&map).unwrap(); std::fs::create_dir_all(&scenery).unwrap();
        std::fs::write(scenery.join("tree.sco"), "[tree]\nleaf.png\n6\n10\n0.3\n0.5\n").unwrap();
        let mut global = String::new();
        for x in [-18, -17, -16] {
            global.push_str(&format!("[map]\n{x}\n-10\ntile_{x}_-10.map\n"));
            std::fs::write(map.join(format!("tile_{x}_-10.map")), "[version]\n14\n[terrain]\n").unwrap();
        }
        std::fs::write(map.join("global.cfg"), global).unwrap();
        let world = World::open(&original, &map.join("global.cfg"), 20000101).unwrap();
        for (x, height) in [(-18, 10.0), (-17, 20.0), (-16, 30.0)] {
            let mut terrain = omsi_map::Terrain::flat(); terrain.heights.fill(height);
            world.data.update(|d| { std::sync::Arc::make_mut(&mut d.ground).terrains.insert((x, -10), std::sync::Arc::new(terrain)); });
        }
        Self { dir, original, content, world }
    }
    fn object(&self, id: i64, x: f64) -> Added {
        Added { template: 0, tile: ((x / 300.0).floor() as i32, -10), id,
            sco: self.original.join("Sceneryobjects/tree.sco"), base: DVec3::new(x, -2800.0, 41.25),
            base_heading: 15.0, moved: DVec3::ZERO, turned: 5.0, deleted: false, standalone: true,
            strings: vec!["leaf2.png".into(), "8".into(), "0.4".into(), "".into()], tilt: [2.0, 3.0],
            shape: Mat4::IDENTITY, ground_height: 20.0, _object_type: None, gpu: None }
    }
    fn save(&self, editor: &Editor) -> Result<Vec<PathBuf>, String> {
        let paths = editor.save(&self.world, "maps/Test/global.cfg", &self.content, &self.original)?;
        // Subsequent saves read the overlay, as they do in the running editor.
        for x in [-18, -17, -16] {
            let path = self.path(x);
            if path.exists() { self.world.register_editor_tile((x, -10), 0, path); }
        }
        Ok(paths)
    }
    fn path(&self, x: i32) -> PathBuf { self.content.join(format!("maps/Test/tile_{x}_-10.map")) }
    fn objects(&self) -> Vec<((i32, i32), omsi_map::MapObject)> {
        let mut out = Vec::new();
        for x in [-18, -17, -16] {
            let p = self.path(x);
            if p.exists() { out.extend(omsi_map::Tile::load(&p).unwrap().objects.into_iter().map(|o| ((x, -10), o))); }
        }
        out
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.dir); } }

#[test]
fn moved_catalogue_trees_cross_negative_tile_boundaries_before_first_save() {
    let f = Fixture::new(); let mut editor = Editor::default();
    for (id, x, delta) in [(11, -4827.0, 86.0), (12, -5102.0, 37.0), (13, -4789.0, -40.0)] {
        let mut a = f.object(id, x); a.moved.x = delta; editor.added.push(a);
    }
    f.save(&editor).unwrap(); let objects = f.objects(); assert_eq!(objects.len(), 3);
    for (tile, o) in objects {
        let a = editor.added.iter().find(|a| a.id == o.id).unwrap();
        let pos = a.base + a.moved;
        assert_eq!(tile, ((pos.x / 300.0).floor() as i32, -10));
        assert_eq!(o.pos[0] + tile.0 as f64 * 300.0, pos.x);
        assert_eq!(o.pos[1] - 3000.0, pos.y);
        let ground = if tile.0 == -16 { 30.0 } else { 20.0 };
        assert_eq!(o.pos[2], pos.z - ground);
        assert_eq!(o.rot, [20.0, 2.0, 3.0]); assert_eq!(o.flag, 4); assert_eq!(o.extra, a.strings[..3]);
    }
    let before: Vec<_> = [-18, -17, -16].map(|x| std::fs::read(f.path(x)).unwrap()).into();
    f.save(&editor).unwrap();
    for (i, x) in [-18, -17, -16].into_iter().enumerate() { assert_eq!(std::fs::read(f.path(x)).unwrap(), before[i]); }
}

#[test]
fn saved_object_moves_twice_deletes_outside_map_and_restores_without_duplicates() {
    let f = Fixture::new(); let mut editor = Editor::default(); editor.added.push(f.object(11, -4900.0));
    for (delta, tile) in [(0.0, -17), (200.0, -16), (-300.0, -18), (0.0, -17)] {
        editor.added[0].moved.x = delta; f.save(&editor).unwrap();
        let objects = f.objects(); assert_eq!(objects.len(), 1); assert_eq!(objects[0].0, (tile, -10));
        assert_eq!(objects[0].1.id, 11);
    }
    editor.added[0].moved.x = 100000.0; editor.added[0].deleted = true;
    f.save(&editor).unwrap(); assert!(f.objects().is_empty());
    editor.added[0].moved.x = 200.0; editor.added[0].deleted = false;
    f.save(&editor).unwrap(); assert_eq!(f.objects().len(), 1); assert_eq!(f.objects()[0].0, (-16, -10));
    for x in [-18, -17, -16] {
        assert_eq!(std::fs::read_to_string(f.original.join(format!("maps/Test/tile_{x}_-10.map"))).unwrap(), "[version]\n14\n[terrain]\n");
    }
}

#[test]
fn missing_target_tile_prevents_any_writes_and_identifies_object() {
    let f = Fixture::new(); let mut editor = Editor::default();
    editor.added.push(f.object(11, -4900.0)); editor.added.push(f.object(12, -4900.0));
    editor.added[1].moved.x = 1000.0;
    let error = f.save(&editor).unwrap_err(); assert!(error.contains("Object 12")); assert!(error.contains("(-13, -10)"));
    assert!(!f.content.exists());
    editor.added[1].deleted = true; f.save(&editor).unwrap(); assert_eq!(f.objects().len(), 1);
}

#[test]
fn roundabout_rules_follow_the_placement_and_leave_no_old_block() {
    let f = Fixture::new(); let mut editor = Editor::default();
    let window = crate::junction_builder::Window::new_roundabout(false);
    std::fs::write(f.original.join("Sceneryobjects/junction.junction.json"), serde_json::to_vec(&window.project).unwrap()).unwrap();
    editor.added.push(f.object(11, -4900.0)); f.save(&editor).unwrap();
    assert!(!f.objects()[0].1.rules.is_empty());
    editor.added[0].moved.x = 200.0; f.save(&editor).unwrap();
    let objects = f.objects(); assert_eq!(objects.len(), 1); assert_eq!(objects[0].0, (-16, -10));
    assert!(!objects[0].1.rules.is_empty());
    let old = std::fs::read(f.path(-17)).unwrap();
    assert!(!decode(&old).0.contains("[openomsi_editor_roundabout_rules]"));
    f.save(&editor).unwrap(); assert_eq!(f.objects().len(), 1);
}
