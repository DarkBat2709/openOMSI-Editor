//! Per-map editor camera bookmarks, kept in user preferences rather than map content.
use serde::{Deserialize, Serialize};
use std::{io::Write, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub fov: f32,
}
impl View {
    fn valid(&self) -> bool {
        self.position.iter().all(|n| n.is_finite())
            && [self.yaw, self.pitch, self.roll, self.fov].iter().all(|n| n.is_finite())
            && (1.0..179.0).contains(&self.fov)
    }
}
#[derive(Serialize, Deserialize)]
struct Bookmark { map: String, view: View }

/// A useful overview even when the stored map camera lies at terrain level.
pub fn start_height(current: f64, terrain: f64) -> f64 {
    let terrain = if terrain.is_finite() { terrain } else { 0.0 };
    if current.is_finite() { current.max(terrain + 35.0) } else { terrain + 35.0 }
}

pub fn key(original: &Path, map: &str) -> String {
    let original = original.canonicalize().unwrap_or_else(|_| original.to_path_buf());
    format!("{}\n{}", original.to_string_lossy(), map.replace('\\', "/").to_lowercase())
}
fn file(dir: &Path, key: &str) -> PathBuf {
    // Stable FNV-1a; the stored full key is also checked, so a hash collision cannot
    // restore another map's position. No user-supplied path enters the filename.
    let hash = key.as_bytes().iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3));
    dir.join(format!("{hash:016x}.json"))
}
pub fn load(dir: &Path, key: &str) -> Option<View> {
    let b: Bookmark = serde_json::from_slice(&std::fs::read(file(dir, key)).ok()?).ok()?;
    (b.map == key && b.view.valid()).then_some(b.view)
}
pub fn save(dir: &Path, key: &str, view: View) -> std::io::Result<()> {
    if !view.valid() { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid editor camera")); }
    std::fs::create_dir_all(dir)?;
    let bytes = serde_json::to_vec(&Bookmark { map: key.into(), view })?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let tmp = dir.join(format!("view-{}-{stamp}.tmp", std::process::id()));
    let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = (|| {
        out.write_all(&bytes)?;
        out.sync_all()?;
        drop(out);
        std::fs::rename(&tmp, file(dir, key))
    })();
    if result.is_err() { let _ = std::fs::remove_file(tmp); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_start_stays_above_elevated_or_lowered_terrain() {
        assert_eq!(start_height(5.0, 200.0), 235.0);
        assert_eq!(start_height(500.0, 200.0), 500.0);
        assert_eq!(start_height(-200.0, -100.0), -65.0);
        assert_eq!(start_height(f64::NAN, 20.0), 55.0);
    }
    #[test]
    fn editor_views_round_trip_update_and_stay_per_map() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("editor-views-{}-{stamp}", std::process::id()));
        let a = key(Path::new("original"), "maps/A/global.cfg");
        let b = key(Path::new("original"), "maps/B/global.cfg");
        assert_eq!(a, key(Path::new("original"), "maps\\A\\global.cfg"));
        assert!(load(&dir, &a).is_none());
        let mut view = View { position: [-1200.5, 832.0, 42.0], yaw: 87.0, pitch: -21.0, roll: 0.0, fov: 60.0 };
        save(&dir, &a, view.clone()).unwrap();
        assert_eq!(load(&dir, &a), Some(view.clone()));
        assert!(load(&dir, &b).is_none());
        view.position[0] = 6789.0;
        save(&dir, &a, view.clone()).unwrap();
        assert_eq!(load(&dir, &a), Some(view.clone()));
        view.yaw = f32::NAN;
        assert!(save(&dir, &a, view).is_err());
        assert!(load(&dir, &a).is_some());
        std::fs::write(file(&dir, &a), b"broken").unwrap();
        assert!(load(&dir, &a).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
