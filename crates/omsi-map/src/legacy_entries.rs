//! Fallback for maps whose entry points exist only as placed scenery objects.
use crate::{global::EntryPoint, GlobalCfg, Tile};
use omsi_cfg::CfgFile;
use std::collections::HashMap;

pub(crate) fn discover(global: &mut GlobalCfg) {
    if !global.entry_points.is_empty() { return; }
    let Some(root) = global.dir().parent().and_then(|maps| maps.parent()) else { return; };
    let root = root.to_path_buf();
    let mut definitions = HashMap::new();
    // Calculate scaling from this map, without changing the process-wide active grid.
    let scaled = global.world_coordinates && omsi_cfg::flags::OMSI_OLD_WORLD_GRID.live_os().is_none();
    let size = crate::world_tile_size(global.tiles.iter().map(|t| t.y));
    for reference in &global.tiles {
        let path = omsi_cfg::resolve_path(global.dir(), &reference.file);
        let Ok(tile) = Tile::load(&path) else { continue; };
        for object in tile.objects {
            let key = object.file.replace('\\', "/").to_ascii_lowercase();
            let is_entry = *definitions.entry(key).or_insert_with(|| {
                let path = omsi_cfg::find_in_roots(&object.file).map(|(_, path)| path)
                    .unwrap_or_else(|| omsi_cfg::resolve_path(&root, &object.file));
                let Ok(file) = CfgFile::read(path) else { return false; };
                let mut reader = file.reader().disabled_blocks();
                while let Some(keyword) = reader.next_keyword() {
                    if keyword == "entrypoint" { return true; }
                }
                false
            });
            if !is_entry { continue; }
            let mut pos = object.pos;
            if scaled {
                let w0 = crate::world_row_width(reference.y);
                let w1 = crate::world_row_width(reference.y + 1);
                pos[0] *= size / ((w0 + w1) / 2.0);
                pos[1] *= size / w1;
            }
            // Object height is terrain-relative. Let the world object index resolve it;
            // it must not be mistaken for an absolute saved entry-point elevation.
            pos[2] = f64::NAN;
            let half = object.rot[0].to_radians() / 2.0;
            let name = object.extra.first().filter(|s| !s.trim().is_empty()).cloned()
                .unwrap_or_else(|| format!("Entry point {}", object.id));
            global.entry_points.push(EntryPoint {
                index: global.entry_points.len() as i32, object_id: object.id,
                unknown: 0, pos, quat: [0.0, half.sin(), 0.0, half.cos()],
                group: reference.index as i32, name,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!("omsi-legacy-entry-{}-{}", std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            std::fs::create_dir_all(path.join("maps/Test")).unwrap();
            std::fs::create_dir_all(path.join("Sceneryobjects/LegacyEntryFixture")).unwrap();
            Self(path)
        }
        fn write(&self, path: &str, text: &str) { std::fs::write(self.0.join(path), text).unwrap(); }
        fn global(&self, prefix: &str) -> GlobalCfg {
            self.write("maps/Test/global.cfg", &format!("{prefix}[map]\n-17\n-10\ntile_-17_-10.map\n[map]\n-17\n-10\ntile_-17_-10.map\n[map]\n-16\n-10\ntile_-16_-10.map\n"));
            let mut global = GlobalCfg::load(&self.0.join("maps/Test/global.cfg")).unwrap();
            global.add_object_entry_points();
            global
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    fn object(file: &str, id: i64, name: &str) -> String {
        format!("[object]\n0\nSceneryobjects\\LegacyEntryFixture\\{file}.sco\n{id}\n10\n20\n0.5\n90\n0\n0\n1\n{name}\n")
    }

    #[test]
    fn discovers_objects_with_labels_and_original_tile_indices() {
        let f = Fixture::new();
        f.write("Sceneryobjects/LegacyEntryFixture/start.sco", "[entrypoint]\n");
        f.write("Sceneryobjects/LegacyEntryFixture/tree.sco", "[friendlyname]\nTree\n");
        let first = format!("[version]\n14\n{}{}", object("START", 7, "Depot"), object("tree", 8, "Tree"));
        f.write("maps/Test/tile_-17_-10.map", &first);
        f.write("maps/Test/tile_-16_-10.map", &format!("[version]\n14\n{}", object("start", 7, "Station")));
        let g = f.global("");
        assert_eq!(g.entry_points.len(), 2);
        assert_eq!((g.entry_points[0].name.as_str(), g.entry_points[1].name.as_str()), ("Depot", "Station"));
        assert_eq!((g.entry_points[0].group, g.entry_points[1].group), (0, 2));
        assert_eq!(&g.entry_points[0].pos[..2], &[10.0, 20.0]);
        assert!(g.entry_points[0].pos[2].is_nan());
        assert!((g.entry_points[0].quat[1] - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        assert_eq!(std::fs::read_to_string(f.0.join("maps/Test/tile_-17_-10.map")).unwrap(), first);
        let world = f.global("[worldcoordinates]\n");
        if omsi_cfg::flags::OMSI_OLD_WORLD_GRID.live_os().is_none() {
            let size = crate::world_tile_size(world.tiles.iter().map(|t| t.y));
            assert!((world.entry_points[0].pos[1] - 20.0 * size / crate::world_row_width(-9)).abs() < 1e-9);
        }
        // A disabled marker is not a spawn object.
        f.write("Sceneryobjects/LegacyEntryFixture/start.sco", "-<DISABLED>-\n[entrypoint]\n-<ENABLED>-\n");
        assert!(f.global("").entry_points.is_empty());
    }

    #[test]
    fn explicit_list_is_authoritative_and_missing_tiles_are_tolerated() {
        let f = Fixture::new();
        assert!(f.global("").entry_points.is_empty());
        let g = f.global("[entrypoints]\n1\n0\n99\n0\n1\n2\n3\n0\n0\n0\n1\n0\nExisting depot\n");
        assert_eq!(g.entry_points.len(), 1);
        assert_eq!(g.entry_points[0].object_id, 99);
        assert_eq!(g.entry_points[0].name, "Existing depot");
        assert!(g.entry_points[0].pos[2].is_finite());
    }
}
