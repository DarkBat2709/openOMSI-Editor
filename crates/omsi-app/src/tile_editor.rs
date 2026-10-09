//! Add empty, edge-matched tiles to a mod copy of an ordinary OMSI map.

use crate::scene::World;
use glam::DVec3;
use hashbrown::HashSet;
use omsi_map::Terrain;
use std::path::{Component, Path, PathBuf};

pub type Key = (i32, i32);
const SIDES: [Key; 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

#[derive(Clone, Copy)]
pub enum Command { Close, Select(Key), Pan(Key), Create, HeightMode(bool), HeightAdjust(f64), EditHeight }

pub struct Window {
    pub center: Key,
    pub selected: Option<Key>,
    pub known: HashSet<Key>,
    pub camera: Key,
    pub bearing: String,
    pub message: String,
    pub rects: Vec<([f32; 4], Command)>,
    pub own_height: bool,
    pub height: f64,
    pub height_edit: Option<String>,
    pub height_replace: bool,
}

pub fn adjacent(known: &HashSet<Key>, key: Key) -> bool {
    SIDES.iter().any(|&(x, y)| known.contains(&(key.0 + x, key.1 + y)))
}

impl Window {
    pub fn new(world: &World, eye: DVec3, yaw: f32) -> Self {
        let known = world.editor_tile_keys();
        let ts = omsi_map::tile_size();
        let camera = ((eye.x / ts).floor() as i32, (eye.y / ts).floor() as i32);
        let start = if known.contains(&camera) || adjacent(&known, camera) { camera }
            else { known.iter().copied().min_by_key(|&(x, y)| {
                (x as i64 - camera.0 as i64).pow(2) + (y as i64 - camera.1 as i64).pow(2)
            }).unwrap_or(camera) };
        let fraction = if start == camera { (eye.x / ts - camera.0 as f64, eye.y / ts - camera.1 as f64) } else { (0.5, 0.5) };
        let selected = suggest(&known, start, yaw, fraction);
        let names = ["North", "Northeast", "East", "Southeast", "South", "Southwest", "West", "Northwest"];
        Self { center: selected.unwrap_or(start), selected, known, camera,
            bearing: names[((yaw.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8].into(),
            message: "Choose a free neighbouring cell. Green = existing, blue = selected.".into(), rects: Vec::new(),
            own_height:false,height:world.editor_terrain_height(eye.x,eye.y).unwrap_or(0.0),height_edit:None,height_replace:true }
    }

    pub fn can_create(&self) -> bool {
        self.height_edit.is_none() && self.selected.is_some_and(|k| !self.known.contains(&k) && adjacent(&self.known, k))
    }

    pub fn hit(&self, cursor: (f32, f32)) -> Option<Command> {
        self.rects.iter().find(|(r, _)| cursor.0 >= r[0] && cursor.0 <= r[2]
            && cursor.1 >= r[1] && cursor.1 <= r[3]).map(|(_, c)| *c)
    }

    pub fn select(&mut self, key: Key) {
        if self.known.contains(&key) {
            self.selected = None;
            self.message = format!("Tile ({}, {}) is already registered.", key.0, key.1);
        } else if !adjacent(&self.known, key) {
            self.selected = None;
            self.message = "The new cell must share an edge with the map.".into();
        } else {
            self.selected = Some(key);
            self.message = if self.own_height {"Set custom height; the new tile will be flat at this height."} else {"Terrain uses neighbouring tile edges. Saved immediately."}.into();
        }
    }
}

/// Follow the horizontal viewing ray through the grid to the first empty neighbour.
fn suggest(known: &HashSet<Key>, start: Key, yaw: f32, fraction: (f64, f64)) -> Option<Key> {
    if !known.contains(&start) && adjacent(known, start) { return Some(start); }
    let (dx, dy) = (yaw as f64).to_radians().sin_cos();
    let (sx, sy) = (if dx >= 0.0 { 1 } else { -1 }, if dy >= 0.0 { 1 } else { -1 });
    let (mut ax, mut ay) = ((if sx > 0 { 1.0 - fraction.0 } else { fraction.0 }) / dx.abs().max(1e-12),
        (if sy > 0 { 1.0 - fraction.1 } else { fraction.1 }) / dy.abs().max(1e-12));
    let mut k = start;
    for _ in 0..128 {
        if ax < ay { k.0 += sx; ax += 1.0 / dx.abs().max(1e-12); }
        else { k.1 += sy; ay += 1.0 / dy.abs().max(1e-12); }
        if !known.contains(&k) { return adjacent(known, k).then_some(k); }
    }
    None
}

/// Fixed boundaries from neighbours; free boundaries have zero outward slope. The
/// interior is harmonic, so it adds no height extrema beyond the existing boundaries.
fn joined_terrain(neighbours: &[Option<Terrain>; 4]) -> Result<Terrain, String> {
    let cells = neighbours.iter().flatten().next().ok_or("No terrain neighbour found")?.cells;
    if cells == 0 || cells > 256 { return Err("Terrain grid not supported (1–256 cells)".into()); }
    let n = cells + 1;
    let mut fixed = vec![None::<f32>; n * n];
    for (side, terrain) in neighbours.iter().enumerate() {
        let Some(t) = terrain else { continue; };
        if t.cells != cells || t.heights.len() != n * n || t.heights.iter().any(|h| !h.is_finite()) {
            return Err("Neighbouring tiles have different or damaged terrain grids".into());
        }
        for j in 0..n {
            let (to, from) = match side {
                0 => (j * n, j * n + cells),
                1 => (j * n + cells, j * n),
                2 => (j, cells * n + j),
                _ => (cells * n + j, j),
            };
            let h = t.heights[from];
            if fixed[to].is_some_and(|old| (old - h).abs() > 0.01) {
                return Err("Neighbouring terrain has conflicting corner heights; smooth and save the shared corner first".into());
            }
            fixed[to].get_or_insert(h);
        }
    }
    let sum: f64 = fixed.iter().flatten().map(|h| *h as f64).sum();
    let mean = (sum / fixed.iter().flatten().count() as f64) as f32;
    let mut h = vec![mean; n * n];
    // One edge can be carried directly across the new field, without a solver or a dip.
    if neighbours.iter().flatten().count() == 1 {
        let side = neighbours.iter().position(Option::is_some).unwrap();
        for y in 0..n { for x in 0..n {
            h[y * n + x] = fixed[match side { 0 => y * n, 1 => y * n + cells, 2 => x, _ => cells * n + x }].unwrap();
        } }
    } else {
        for (i, v) in fixed.iter().enumerate() { if let Some(v) = v { h[i] = *v; } }
        // Red/black Gauss-Seidel stays between neighbour heights and avoids large buffers.
        for _ in 0..2000 {
            let mut delta = 0.0_f32;
            for parity in 0..2 {
                for y in 0..n { for x in 0..n {
                    let i = y * n + x;
                    if (x + y) % 2 != parity || fixed[i].is_some() { continue; }
                    let mut sum = 0.0; let mut count = 0.0;
                    if x > 0 { sum += h[i - 1]; count += 1.0; }
                    if x < cells { sum += h[i + 1]; count += 1.0; }
                    if y > 0 { sum += h[i - n]; count += 1.0; }
                    if y < cells { sum += h[i + n]; count += 1.0; }
                    let next = sum / count;
                    delta = delta.max((next - h[i]).abs()); h[i] = next;
                } }
            }
            if delta < 0.0001 { break; }
        }
    }
    Ok(Terrain { cells, heights: h })
}

/// Keep the original encoding, comments, entry points and every existing tile ordinal.
fn append_global(bytes: &[u8], key: Key, file: &str) -> Result<(Vec<u8>, usize), String> {
    let (mut text, enc) = crate::editor::decode(bytes);
    let cfg = omsi_map::GlobalCfg::parse(&omsi_cfg::CfgFile::from_str("global.cfg", text.trim_start_matches('\u{feff}')));
    if cfg.world_coordinates { return Err("New tiles are currently supported only for standard OMSI maps".into()); }
    if cfg.raw_tiles.contains(&key) || cfg.tiles.iter().any(|t| t.file.eq_ignore_ascii_case(file)) {
        return Err("This tile is already registered in global.cfg".into());
    }
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let entry = format!("{nl}{nl}[map]{nl}{}{nl}{}{nl}{file}{nl}", key.0, key.1);
    text.push_str(&entry);
    Ok((crate::editor::encode(&text, enc), cfg.raw_tiles.len()))
}

fn mod_dir(content: &Path, map_rel: &str, original: &Path) -> Result<PathBuf, String> {
    let rel = Path::new(map_rel);
    if rel.is_absolute() || rel.components().any(|c| !matches!(c, Component::Normal(_)))
        || !rel.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("global.cfg")) {
        return Err("The map must be opened with a relative path such as maps/MapName/global.cfg".into());
    }
    let original = original.canonicalize().map_err(|e| e.to_string())?;
    let mut ancestor = content;
    while !ancestor.exists() { ancestor = ancestor.parent().ok_or("Content folder has no existing parent folder")?; }
    if ancestor.canonicalize().map_err(|e| e.to_string())?.starts_with(&original) {
        return Err("The content folder is inside the original installation".into());
    }
    std::fs::create_dir_all(content).map_err(|e| e.to_string())?;
    let base = content.canonicalize().map_err(|e| e.to_string())?;
    if base.starts_with(&original) { return Err("The content folder is inside the original installation".into()); }
    let mut dir = base.clone();
    for part in rel.parent().unwrap_or(Path::new("")).components() {
        dir = local_path(&dir, &part.as_os_str().to_string_lossy())?;
        match std::fs::create_dir(&dir) {
            Ok(()) => {},
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(e) => return Err(e.to_string()),
        }
        dir = dir.canonicalize().map_err(|e| e.to_string())?;
        if !dir.starts_with(&base) || dir.starts_with(&original) { return Err("Map folder points outside the content folder".into()); }
    }
    Ok(dir)
}

/// Resolve only inside the writable directory, never through another content root.
fn local_path(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let mut found = None;
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
            if found.is_some() { return Err(format!("Ambiguous letter case for {name}")); }
            found = Some(entry.path());
        }
    }
    Ok(found.unwrap_or_else(|| dir.join(name)))
}

fn write_new(path: &Path, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Err(e) = f.write_all(data).and_then(|_| f.sync_all()) {
        drop(f); let _ = std::fs::remove_file(path); return Err(e.to_string());
    }
    Ok(())
}

/// The global file is the commit point. Failed writes remove only the files we created.
fn write_set(dir: &Path, key: Key, terrain: &Terrain, old_global: &[u8], new_global: &[u8]) -> Result<PathBuf, String> {
    let file = format!("tile_{}_{}.map", key.0, key.1);
    let map = dir.join(&file); let ground = dir.join(format!("{file}.terrain"));
    let global = local_path(dir,"global.cfg")?;
    if std::fs::symlink_metadata(&global).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err("global.cfg is not a regular file".into());
    }
    let previous = match std::fs::read(&global) {
        Ok(b) => Some(b), Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    if previous.as_deref().is_some_and(|b| b != old_global) { return Err("global.cfg has changed; reopen the window".into()); }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let temp = dir.join(format!("global.cfg.editor-{stamp}.tmp"));
    let backup = dir.join(format!("global.cfg.before-editor-{stamp}"));
    // Only ASCII keywords in a new tile; OMSI's native UTF-16 encoding and CRLF.
    let tile = crate::editor::encode("[version]\r\n14\r\n\r\n[terrain]\r\n\r\n[variable_terrain]\r\n", crate::editor::Encoding::Utf16Le);
    write_new(&temp, new_global)?;
    let mut made_map = false; let mut made_ground = false;
    let result = (|| {
        write_new(&map, &tile)?; made_map = true;
        write_new(&ground, &terrain.to_bytes())?; made_ground = true;
        write_new(&backup, old_global)?;
        let now = match std::fs::read(&global) {
            Ok(b) => Some(b), Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if now != previous { return Err("global.cfg changed while saving".into()); }
        std::fs::rename(&temp, &global).map_err(|e| e.to_string())
    })();
    if let Err(e) = result {
        if made_map { let _ = std::fs::remove_file(&map); }
        if made_ground { let _ = std::fs::remove_file(&ground); }
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    Ok(map)
}

pub fn create(world: &World, key: Key, content: &Path, map_rel: &str, original: &Path, height:Option<f64>) -> Result<PathBuf, String> {
    if height.is_some_and(|h|!h.is_finite() || !(crate::terrain_editor::HEIGHT_MIN..=crate::terrain_editor::HEIGHT_MAX).contains(&h)) {
        return Err("Invalid tile height".into());
    }
    if omsi_map::world_coordinates() || world.global.world_coordinates { return Err("Tile creation requires a standard OMSI map".into()); }
    let known = world.editor_tile_keys();
    if known.contains(&key) { return Err("This tile already exists".into()); }
    if !adjacent(&known, key) { return Err("New tile must directly border the map".into()); }
    let dir = mod_dir(content, map_rel, original)?;
    let filename = format!("tile_{}_{}.map", key.0, key.1);
    for ext in ["", ".terrain", ".water"] {
        let name = format!("{filename}{ext}");
        if omsi_cfg::vfs::exists(&omsi_cfg::resolve_path(&dir, &name))
            || omsi_cfg::vfs::exists(&omsi_cfg::resolve_path(&world.map_dir, &name)) {
            return Err(format!("{name} already exists; no overwrite"));
        }
    }
    let global_path = local_path(&dir,"global.cfg")?;
    let bytes = if global_path.exists() { std::fs::read(&global_path).map_err(|e| e.to_string())? }
        else { omsi_cfg::vfs::read(&world.global.path).map_err(|e| e.to_string())? };
    let (text, _) = crate::editor::decode(&bytes);
    let current = omsi_map::GlobalCfg::parse(&omsi_cfg::CfgFile::from_str("global.cfg", text.trim_start_matches('\u{feff}')));
    if current.tiles.iter().map(|t| (t.x, t.y)).collect::<HashSet<_>>() != known
        || !current.raw_tiles.starts_with(&world.global.raw_tiles)
        || world.global.tiles.iter().any(|old| !current.tiles.iter().any(|t| t == old)) {
        return Err("Tile list in global.cfg changed outside the editor; reload map".into());
    }
    let mut neighbours: [Option<Terrain>; 4] = [None, None, None, None];
    for (side, &(dx, dy)) in SIDES.iter().enumerate() {
        let k = (key.0 + dx, key.1 + dy);
        if !known.contains(&k) { continue; }
        let src = world.tile_source(k.0, k.1).ok_or("Neighbouring tile has no file")?;
        if !omsi_cfg::vfs::is_file(&src) { return Err("Neighbouring tile file missing".into()); }
        let mod_ground = local_path(&dir,&format!("{}.terrain", src.file_name().ok_or("Neighbouring tile has no filename")?.to_string_lossy()))?;
        let path = if mod_ground.is_file() { mod_ground } else { crate::scene::tile_companion(&src, ".terrain") };
        let terrain = if omsi_cfg::vfs::is_file(&path) {
            Terrain::load(&path).map_err(|e| format!("Terrain ({}, {}): {e}", k.0, k.1))?
        } else { Terrain::flat() };
        if height.is_none() && world.terrain_edits.lock().get(&k).is_some_and(|edited| edited != &terrain) {
            return Err("Neighbouring tile terrain was edited: Ctrl+S to save first, then create tile".into());
        }
        neighbours[side] = Some(terrain);
    }
    let terrain = if let Some(height)=height {
        let cells=neighbours.iter().flatten().next().map_or(60,|t|t.cells);
        if cells==0 || cells>256 {return Err("Terrain grid not supported".into());}
        Terrain {cells,heights:vec![height as f32;(cells+1)*(cells+1)]}
    } else {joined_terrain(&neighbours)?};
    let (global, index) = append_global(&bytes, key, &filename)?;
    let path = write_set(&dir, key, &terrain, &bytes, &global)?;
    world.register_editor_tile(key, index, path.clone());
    log::info!("tile editor: created ({}, {}) at {}, global.cfg index {index}", key.0, key.1, path.display());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ground(cells: usize, f: impl Fn(usize, usize) -> f32) -> Terrain {
        Terrain { cells, heights: (0..=cells).flat_map(|y| (0..=cells).map(move |x| (x, y))).map(|(x,y)| f(x,y)).collect() }
    }
    #[test]
    fn one_edge_carries_its_profile_without_a_seam_or_new_hill() {
        for side in 0..4 {
            let mut neighbours = [None, None, None, None];
            neighbours[side] = Some(ground(8, |x, y| 30.0 + x as f32 * 0.2 + y as f32 * 0.3));
            let t = joined_terrain(&neighbours).unwrap();
            for y in 0..9 { for x in 0..9 {
                let (sx, sy) = match side { 0 => (8,y), 1 => (0,y), 2 => (x,8), _ => (x,0) };
                assert_eq!(t.height_at(x,y), neighbours[side].as_ref().unwrap().height_at(sx,sy));
            } }
        }
    }
    #[test]
    fn opposite_edges_form_a_ramp_and_keep_exact_boundary_samples() {
        let t = joined_terrain(&[Some(ground(8, |_,_| 40.0)), Some(ground(8, |_,_| 48.0)), None, None]).unwrap();
        for y in 0..9 {
            assert_eq!(t.height_at(0,y), 40.0); assert_eq!(t.height_at(8,y), 48.0);
            for x in 0..9 { assert!((t.height_at(x,y) - (40.0 + x as f32)).abs() < 0.01); }
        }
        assert!(joined_terrain(&[Some(ground(8, |_,_|40.0)), None, Some(ground(8, |_,_|50.0)), None]).is_err());
    }
    #[test]
    fn append_preserves_encoding_bytes_and_repeated_tile_indices() {
        let text = "[friendlyname]\r\nStraße\r\n[map]\r\n0\r\n0\r\ntile_0_0.map\r\n[map]\r\n0\r\n0\r\ntile_0_0.map\r\n";
        for enc in [crate::editor::Encoding::Utf8, crate::editor::Encoding::Latin1, crate::editor::Encoding::Utf16Le] {
            let bytes = crate::editor::encode(text, enc);
            let (updated, index) = append_global(&bytes, (1,0), "tile_1_0.map").unwrap();
            assert!(updated.starts_with(&bytes)); assert_eq!(index,2);
            let cfg = omsi_map::GlobalCfg::parse(&omsi_cfg::CfgFile::from_str("global.cfg", &crate::editor::decode(&updated).0));
            assert_eq!(cfg.tiles[1].index,2); assert_eq!(cfg.raw_tiles.len(),3);
            assert!(append_global(&updated,(1,0),"tile_1_0.map").is_err());
        }
    }
    #[test]
    fn viewing_direction_proposes_a_free_adjacent_tile() {
        let known = [(0,0),(0,1)].into_iter().collect();
        assert_eq!(suggest(&known,(0,0),0.0,(0.5,0.5)),Some((0,2)));
        assert_eq!(suggest(&known,(0,0),90.0,(0.5,0.5)),Some((1,0)));
        assert_eq!(suggest(&known,(0,0),270.0,(0.5,0.5)),Some((-1,0)));
        assert!(!adjacent(&known,(1,2)));
    }
    #[test]
    fn failed_set_rolls_back_new_map_and_keeps_conflicting_file() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("openomsi-tile-test-{}-{stamp}",std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("tile_1_0.map.terrain"), b"keep").unwrap();
        let old = b"[map]\n0\n0\ntile_0_0.map\n";
        assert!(write_set(&dir,(1,0),&Terrain::flat(),old,b"new global").is_err());
        assert!(!dir.join("tile_1_0.map").exists());
        assert_eq!(std::fs::read(dir.join("tile_1_0.map.terrain")).unwrap(),b"keep");
        assert!(!dir.join("global.cfg").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn created_tiles_are_live_persistent_and_never_replace_the_original() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("openomsi-tile-world-test-{}-{stamp}",std::process::id()));
        let original = dir.join("original"); let content = dir.join("content");
        let map = original.join("maps/Test"); std::fs::create_dir_all(&map).unwrap();
        let cfg = b"[name]\nTest\n[map]\n0\n0\ntile_0_0.map\n[map]\n0\n0\ntile_0_0.map\n";
        std::fs::write(map.join("global.cfg"),cfg).unwrap();
        let tile = b"[version]\n14\n[terrain]\n";
        std::fs::write(map.join("tile_0_0.map"),tile).unwrap();
        let terrain = ground(60,|_,y| 40.0 + y as f32 * 0.1);
        std::fs::write(map.join("tile_0_0.map.terrain"),terrain.to_bytes()).unwrap();
        let world = World::open(&original,&map.join("global.cfg"),20000101).unwrap();
        world.layout();
        let path = create(&world,(1,0),&content,"maps/Test/global.cfg",&original,None).unwrap();
        assert_eq!(world.tile_source(1,0),Some(path.clone()));
        assert!(world.has_tile((1,0)));
        assert!(world.select_tiles(Some((1,0)),Some(0)).iter().any(|t| (t.0,t.1)==(1,0)));
        let created = omsi_map::Tile::load(&path).unwrap();
        assert_eq!(created.version,14); assert!(created.has_terrain);
        assert!(created.splines.is_empty() && created.objects.is_empty());
        let ground_path = PathBuf::from(format!("{}.terrain",path.display()));
        assert_eq!(Terrain::load(&ground_path).unwrap().height_at(0,20),42.0);
        let global_path = content.join("maps/Test/global.cfg");
        let saved = std::fs::read(&global_path).unwrap();
        assert!(saved.starts_with(cfg));
        assert!(create(&world,(1,0),&content,"maps/Test/global.cfg",&original,None).is_err());
        assert_eq!(std::fs::read(&global_path).unwrap(),saved);
        create(&world,(2,0),&content,"maps/Test/global.cfg",&original,None).unwrap();
        let reopened = omsi_map::GlobalCfg::load(&global_path).unwrap();
        assert_eq!(reopened.tiles.iter().map(|t|t.index).collect::<Vec<_>>(),vec![0,2,3]);
        let raised=create(&world,(1,1),&content,"maps/Test/global.cfg",&original,Some(65.5)).unwrap();
        let t=Terrain::load(&PathBuf::from(format!("{}.terrain",raised.display()))).unwrap();
        assert!(t.heights.iter().all(|h|*h==65.5));
        assert!(create(&world,(2,1),&content,"maps/Test/global.cfg",&original,Some(f64::NAN)).is_err());
        assert_eq!(std::fs::read(map.join("global.cfg")).unwrap(),cfg);
        assert_eq!(std::fs::read(map.join("tile_0_0.map")).unwrap(),tile);
        assert!(mod_dir(&original,"maps/Test/global.cfg",&original).is_err());
        assert!(mod_dir(&content,"../Test/global.cfg",&original).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
