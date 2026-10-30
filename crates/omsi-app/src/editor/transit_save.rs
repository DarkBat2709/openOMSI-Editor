//! Keep placed stop/entry helpers and the OMSI timetable indexes in step on Save.
use super::*;
use omsi_timetable::BusStopEntry;

pub(super) fn save(
    ed: &Editor,
    world: &World,
    map_rel: &str,
    content: &Path,
    original: &Path,
    tiles: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let rel = Path::new(map_rel).parent().ok_or("Map has no folder")?;
    let folder = content.join(rel);
    let global_out = folder.join("global.cfg");
    protect_output(&global_out, original)?;
    let global_source = if global_out.is_file() {
        global_out.clone()
    } else {
        world.global.path.clone()
    };
    let bytes = std::fs::read(&global_source).map_err(|e| e.to_string())?;
    let text = omsi_cfg::decode_text(&bytes);
    let mut global =
        omsi_map::GlobalCfg::parse(&omsi_cfg::CfgFile::from_str(&global_source, &text));
    let deleted: HashSet<i64> = world
        .object_edits
        .lock()
        .iter()
        .filter(|(_, e)| e.deleted)
        .map(|(id, _)| *id)
        .chain(ed.added.iter().filter(|a| a.deleted).map(|a| a.id))
        .collect();
    let source_dir = world.global.path.parent().ok_or("Map has no folder")?;
    let stops_path = folder.join("TTData/Busstops.cfg");
    let stops_source = if stops_path.is_file() {
        stops_path.clone()
    } else {
        omsi_cfg::resolve_path(source_dir, "TTData/Busstops.cfg")
    };
    let mut stops = if stops_source.exists() {
        let f = omsi_cfg::CfgFile::read(&stops_source).map_err(|e| e.to_string())?;
        omsi_timetable::parse_busstops(&f)
    } else {
        Vec::new()
    };
    let old_stops = stops.clone();
    let old_entries = global.entry_points.clone();
    stops.retain(|s| !deleted.contains(&s.object_id));
    global
        .entry_points
        .retain(|e| !deleted.contains(&e.object_id));
    for path in tiles
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "map"))
    {
        let Some(reference) = global.tiles.iter().find(|t| {
            path.file_name()
                .is_some_and(|n| n == std::ffi::OsStr::new(&t.file))
        }) else {
            continue;
        };
        let tile = omsi_map::Tile::load(path).map_err(|e| e.to_string())?;
        for o in tile.objects {
            let Ok(ty) = world.editor_object_type(&o.file) else {
                continue;
            };
            if !ty.sco.is_bus_stop && !ty.sco.is_entry_point {
                continue;
            }
            let name = o
                .extra
                .first()
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| {
                    format!(
                        "{} {}",
                        if ty.sco.is_bus_stop { "Stop" } else { "Entry" },
                        o.id
                    )
                });
            if ty.sco.is_bus_stop {
                if let Some(stop) = stops.iter_mut().find(|s| s.object_id == o.id) {
                    stop.name = name.clone();
                } else {
                    stops.push(BusStopEntry {
                        name: name.clone(),
                        object_id: o.id,
                        ..Default::default()
                    });
                }
            }
            if ty.sco.is_entry_point {
                let half = o.rot[0].to_radians() / 2.0;
                let mut pos = o.pos;
                if !ty.sco.absolute_height() {
                    pos[2] += world
                        .ground_terrain(
                            pos[0] + reference.x as f64 * omsi_map::tile_size(),
                            pos[1] + reference.y as f64 * omsi_map::tile_size(),
                        )
                        .unwrap_or(0.0);
                }
                let index = global
                    .entry_points
                    .iter()
                    .map(|e| e.index)
                    .max()
                    .unwrap_or(-1)
                    + 1;
                let mut entry = omsi_map::global::EntryPoint {
                    index,
                    object_id: o.id,
                    unknown: 0,
                    pos,
                    quat: [0.0, half.sin(), 0.0, half.cos()],
                    group: reference.index as i32,
                    name,
                };
                if let Some(old) = global.entry_points.iter_mut().find(|e| e.object_id == o.id) {
                    entry.index = old.index;
                    *old = entry;
                } else {
                    global.entry_points.push(entry);
                }
            }
        }
    }
    let mut saved = Vec::new();
    if stops != old_stops {
        ensure_ttdata(&folder, source_dir, original)?;
        save_copy(&stops_path, &omsi_cfg::encode_text(&stops_text(&stops)))?;
        saved.push(stops_path);
    }
    if global.entry_points != old_entries {
        let block = entries_text(&global.entry_points);
        let rewritten = replace_block(&text, "entrypoints", &block);
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        save_copy(&global_out, &omsi_cfg::encode_text(&rewritten))?;
        saved.push(global_out);
    }
    if !saved.is_empty() {
        omsi_cfg::content_changed();
    }
    Ok(saved)
}
pub(crate) fn ensure_ttdata(
    folder: &Path,
    source: &Path,
    original: &Path,
) -> Result<PathBuf, String> {
    let target = folder.join("TTData");
    protect_output(&target, original)?;
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let source = omsi_cfg::resolve_path(source, "TTData");
    if source != target {
        if let Ok(files) = std::fs::read_dir(&source) {
            for f in files.flatten() {
                let path = f.path();
                let out = target.join(f.file_name());
                if path.is_file() && !out.exists() {
                    std::fs::copy(path, out).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    Ok(target)
}
pub(crate) fn stops_text(stops: &[BusStopEntry]) -> String {
    let mut s = String::new();
    for b in stops {
        s += &format!(
            "[busstop]\n{}\n{}\n{}\n{}\n{}\n{}\n\n",
            b.name, b.group, b.object_id, b.params[0], b.params[1], b.params[2]
        );
    }
    s
}
fn entries_text(entries: &[omsi_map::global::EntryPoint]) -> String {
    let mut s = format!("[entrypoints]\n{}\n", entries.len());
    for e in entries {
        s += &format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            e.index,
            e.object_id,
            e.unknown,
            e.pos[0],
            e.pos[2],
            e.pos[1],
            e.quat[0],
            e.quat[1],
            e.quat[2],
            e.quat[3],
            e.group,
            e.name
        );
    }
    s.push('\n');
    s
}
fn replace_block(text: &str, key: &str, new: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    let mut found = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            skip = t.eq_ignore_ascii_case(&format!("[{key}]"));
            if skip {
                if !found {
                    out.push_str(new);
                    found = true;
                }
                continue;
            }
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !found {
        out.push('\n');
        out.push_str(new);
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entries_round_trip_and_preserve_other_sections() {
        let e = omsi_map::global::EntryPoint {
            index: 7,
            object_id: 987,
            pos: [12.0, 25.0, 3.0],
            quat: [0.0, 0.5, 0.0, 0.866],
            group: 3,
            name: "Hauptbahnhof".into(),
            ..Default::default()
        };
        let text = replace_block(
            "[name]\nTest\n[entrypoints]\n0\n[map]\n0\n0\ntile_0_0.map\n",
            "entrypoints",
            &entries_text(&[e.clone()]),
        );
        let g = omsi_map::GlobalCfg::parse(&omsi_cfg::CfgFile::from_str("global.cfg", &text));
        assert_eq!(g.entry_points, vec![e]);
        assert_eq!(g.name, "Test");
        assert_eq!(g.tiles.len(), 1);
    }
    #[test]
    fn stop_names_ids_and_parameters_round_trip() {
        let s = BusStopEntry {
            name: "Königsplatz".into(),
            group: 3,
            object_id: 75,
            params: [1.0, 2.0, 3.0],
        };
        assert_eq!(
            omsi_timetable::parse_busstops(&omsi_cfg::CfgFile::from_str(
                "Busstops.cfg",
                &stops_text(&[s.clone()])
            )),
            vec![s]
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn placed_helpers_are_registered_renamed_and_removed_on_save() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("editor-transit-{stamp}"));
        let root = base.join("original");
        let map = root.join("maps/Test");
        let content = base.join("content");
        let out = content.join("maps/Test");
        std::fs::create_dir_all(&map).unwrap();
        std::fs::create_dir_all(root.join("Sceneryobjects")).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            root.join("Sceneryobjects/stop.sco"),
            "[busstop]\n[onlyeditor]\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Sceneryobjects/start.sco"),
            "[entrypoint]\n[onlyeditor]\n",
        )
        .unwrap();
        let global = "[name]\nTest\n[map]\n0\n0\ntile_0_0.map\n";
        std::fs::write(map.join("global.cfg"), global).unwrap();
        std::fs::write(map.join("tile_0_0.map"), "[version]\n14\n").unwrap();
        let world = World::open(&root, &map.join("global.cfg"), 20000101).unwrap();
        let ed = Editor::default();
        let tile = out.join("tile_0_0.map");
        let record = |file: &str, id: i64, name: &str| {
            format!(
                "[object]\n0\nSceneryobjects/{file}.sco\n{id}\n10\n20\n3\n90\n0\n0\n1\n{name}\n"
            )
        };
        let text = format!(
            "[version]\n14\n{}{}",
            record("stop", 101, "Hauptbahnhof"),
            record("start", 102, "Betriebshof")
        );
        std::fs::write(&tile, &text).unwrap();
        save(
            &ed,
            &world,
            "maps/Test/global.cfg",
            &content,
            &root,
            &[tile.clone()],
        )
        .unwrap();
        let load_stops = || {
            omsi_timetable::parse_busstops(
                &omsi_cfg::CfgFile::read(&out.join("TTData/Busstops.cfg")).unwrap(),
            )
        };
        assert_eq!(load_stops()[0].name, "Hauptbahnhof");
        let g = omsi_map::GlobalCfg::load(&out.join("global.cfg")).unwrap();
        assert_eq!(g.entry_points.len(), 1);
        assert_eq!(g.entry_points[0].object_id, 102);
        assert_eq!(g.entry_points[0].name, "Betriebshof");
        std::fs::write(&tile, text.replace("Hauptbahnhof", "Bahnhof")).unwrap();
        save(
            &ed,
            &world,
            "maps/Test/global.cfg",
            &content,
            &root,
            &[tile.clone()],
        )
        .unwrap();
        assert_eq!(load_stops()[0].name, "Bahnhof");
        world.object_edits.lock().entry(101).or_default().deleted = true;
        world.object_edits.lock().entry(102).or_default().deleted = true;
        std::fs::write(&tile, "[version]\n14\n").unwrap();
        save(
            &ed,
            &world,
            "maps/Test/global.cfg",
            &content,
            &root,
            &[tile],
        )
        .unwrap();
        assert!(load_stops().is_empty());
        assert!(
            omsi_map::GlobalCfg::load(&out.join("global.cfg"))
                .unwrap()
                .entry_points
                .is_empty()
        );
        assert_eq!(
            std::fs::read_to_string(map.join("global.cfg")).unwrap(),
            global
        );
        std::fs::remove_dir_all(base).unwrap();
    }
}
