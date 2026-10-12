//! New editor maps contain only generated terrain and an original, plain colour texture.
//! The destination directory is reserved before writing: existing maps are never replaced.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub fn validate_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.len() > 80 || name.ends_with('.')
        || name.chars().any(|c| c.is_control() || "<>:\"/\\|?*[]".contains(c))
        || name == "." || name == ".."
    {
        bail!("Enter a map name without special characters such as /, \\, : or [] (max. 80 bytes).");
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        bail!("This map name is reserved on Windows.");
    }
    Ok(name)
}

fn text_file(text: &str) -> Vec<u8> {
    // OMSI reads Unicode configuration files as UTF-16LE with BOM.
    let mut bytes = vec![0xff, 0xfe];
    for unit in text.encode_utf16() { bytes.extend_from_slice(&unit.to_le_bytes()); }
    bytes
}

fn ground_bitmap() -> Vec<u8> {
    // A generated 2x2 RGB BMP; no original OMSI assets are copied into the map.
    let mut b = vec![0u8; 70];
    b[..2].copy_from_slice(b"BM");
    b[2..6].copy_from_slice(&70u32.to_le_bytes());
    b[10..14].copy_from_slice(&54u32.to_le_bytes());
    b[14..18].copy_from_slice(&40u32.to_le_bytes());
    b[18..22].copy_from_slice(&2i32.to_le_bytes());
    b[22..26].copy_from_slice(&2i32.to_le_bytes());
    b[26..28].copy_from_slice(&1u16.to_le_bytes());
    b[28..30].copy_from_slice(&24u16.to_le_bytes());
    b[34..38].copy_from_slice(&16u32.to_le_bytes());
    for i in [54, 57, 62, 65] { b[i..i+3].copy_from_slice(&[72, 112, 86]); }
    b
}

// Inspect each physical root independently; the normal resolver merges mod overlays.
fn maps_folder(root: &Path) -> PathBuf {
    std::fs::read_dir(root).ok().and_then(|entries| entries.flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("maps"))
        .map(|e| e.path())).unwrap_or_else(|| root.join("maps"))
}

/// `content` must be the registered writable content folder; maps must remain below it
/// so reopening, texture lookup and the existing tile editor use the same relative path.
pub fn create(content: &Path, original: &Path, name: &str, author: &str, description: &str) -> Result<String> {
    let name = validate_name(name)?;
    if author.len() > 160 || description.len() > 2000 || [author, description].iter().any(|s| s.chars().any(|c| c.is_control() || c == '[' || c == ']')) {
        bail!("Author and description must not contain control characters or square brackets (max. 160 / 2000 bytes).");
    }
    let content = content.canonicalize().context("Game content folder is not accessible")?;
    let original = original.canonicalize().context("OMSI 2 folder is not accessible")?;
    if content.starts_with(&original) {
        bail!("New maps must be outside the original OMSI 2 installation.");
    }
    let maps = maps_folder(&content);
    std::fs::create_dir_all(&maps)?;
    let maps_real = maps.canonicalize()?;
    if !maps_real.starts_with(&content) || maps_real.starts_with(&original) {
        bail!("The maps folder points outside the game content folder.");
    }
    let relative = format!("maps/{name}/global.cfg");
    // Also reject case-only clashes on Linux, keeping names portable to Windows.
    for root in [&content, &original] {
        let map_root = maps_folder(root);
        if let Ok(entries) = std::fs::read_dir(map_root) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().to_lowercase() == name.to_lowercase() {
                    bail!("A map or folder with this name already exists.");
                }
            }
        }
    }
    let dir = maps.join(name);
    std::fs::create_dir(&dir).context("Could not create map folder")?;
    let result = write_map(&dir, name, author, description);
    if let Err(error) = result {
        // Only remove files belonging to this attempt, never a pre-existing directory.
        for file in ["global.cfg", "tile_0_0.map", "tile_0_0.map.terrain", "editor-ground.bmp"] {
            let _ = std::fs::remove_file(dir.join(file));
        }
        let _ = std::fs::remove_dir(&dir);
        return Err(error);
    }
    omsi_cfg::content_changed();
    Ok(relative)
}

fn write_map(dir: &Path, name: &str, author: &str, description: &str) -> Result<()> {
    use std::io::Write;
    fn write(path: PathBuf, bytes: &[u8]) -> Result<()> {
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        Ok(())
    }
    write(dir.join("editor-ground.bmp"), &ground_bitmap())?;
    write(dir.join("tile_0_0.map"), &text_file("[version]\r\n14\r\n\r\n[terrain]\r\n\r\n[variable_terrain]\r\n"))?;
    write(dir.join("tile_0_0.map.terrain"), &omsi_map::Terrain::flat().to_bytes())?;
    let description = if author.trim().is_empty() { description.trim().to_string() } else { format!("Autor: {}\r\n{}", author.trim(), description.trim()) };
    let global = format!("[version]\r\n14\r\n[name]\r\n{name}\r\n[friendlyname]\r\n{name}\r\n[description]\r\n{description}\r\n[end]\r\n[nextIDCode]\r\n1\r\n[mapcam]\r\n0\r\n0\r\n150\r\n30\r\n150\r\n0\r\n-30\r\n0\r\n[groundtex]\r\nmaps/{name}/editor-ground.bmp\r\nmaps/{name}/editor-ground.bmp\r\n0\r\n12\r\n12\r\n[map]\r\n0\r\n0\r\ntile_0_0.map\r\n");
    // Publish the global file last: only complete maps are discoverable by the launcher.
    write(dir.join("global.cfg"), &text_file(&global))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_and_windows_reserved_names_are_rejected() {
        for name in ["", "../old", "a/b", "a\\b", "[map]", "a\nb", "CON", "aux.txt", "LPT1", "name."] {
            assert!(validate_name(name).is_err(), "{name:?}");
        }
        assert_eq!(validate_name("  Neue Karte ä  ").unwrap(), "Neue Karte ä");
    }

    #[test]
    fn new_map_round_trips_and_existing_maps_are_preserved() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let temp = std::env::temp_dir().join(format!("editor-new-map-{}-{stamp}", std::process::id()));
        let content = temp.join("content");
        let original = temp.join("original");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::create_dir_all(original.join("Maps/Stock")).unwrap();
        let rel = create(&content, &original, "Neue Karte ä", "Chris", "Eine neue Karte").unwrap();
        let global = content.join(&rel);
        let bytes = std::fs::read(&global).unwrap();
        let cfg = omsi_map::GlobalCfg::parse(&omsi_cfg::CfgFile::from_bytes(&global, &bytes));
        assert_eq!(cfg.name, "Neue Karte ä");
        assert!(cfg.description.contains("Autor: Chris"));
        assert!(cfg.description.contains("Eine neue Karte"));
        assert_eq!(cfg.tiles.len(), 1);
        assert_eq!(cfg.ground_textures.len(), 1);
        assert!(content.join(&cfg.ground_textures[0].texture).is_file());
        let dir = global.parent().unwrap();
        let tile = omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_bytes(dir.join("tile_0_0.map"), &std::fs::read(dir.join("tile_0_0.map")).unwrap()));
        assert!(tile.has_terrain);
        let terrain = omsi_map::Terrain::parse(&std::fs::read(dir.join("tile_0_0.map.terrain")).unwrap()).unwrap();
        assert!(terrain.heights.iter().all(|h| *h == 0.0));
        assert!(create(&content, &original, "neue karte ä", "", "").is_err());
        assert!(create(&content, &original, "Stock", "", "").is_err());
        assert!(create(&content, &original, "Injection", "", "[end]").is_err());
        assert!(!content.join("maps/Injection").exists());
        assert!(create(&original, &original, "No", "", "").is_err());
        assert_eq!(std::fs::read(&global).unwrap(), bytes);
        std::fs::remove_dir_all(temp).unwrap();
    }
}
