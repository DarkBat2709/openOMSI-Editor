/// A route helper's text in letters its font lacks breaks its lines at `@` as OMSI's
/// own text textures do (#1553): two bands of letters, one above the other, no `@`.
#[test]
fn a_long_helper_text_is_made_smaller_before_it_is_narrowed() {
    let tt = omsi_model::TextTexture { width: 64, height: 128, color: [255.0; 3], ..Default::default() };
    let ink_rows = |text: &str| -> usize {
        let img = helper_text_image(&tt, None, text).expect("drawn with the interface font");
        (0..128).filter(|&y| (0..64).any(|x| img.rgba[(y * 64 + x) * 4 + 3] > 0)).count()
    };
    // (#1696: the long name keeps letters of a readable shape: smaller, not only thinner)
    assert!(ink_rows("Łąka Łąka Łąka Łąka Łąka") < ink_rows("Łąka"));
}

#[test]
fn helper_text_breaks_lines_at_the_at_sign() {
    let tt = omsi_model::TextTexture { width: 256, height: 64, color: [255.0; 3], ..Default::default() };
    let rows = |text: &str| -> Vec<bool> {
        let img = helper_text_image(&tt, None, text).expect("drawn with the interface font");
        (0..64).map(|y| (0..256).any(|x| img.rgba[(y * 256 + x) * 4 + 3] > 0)).collect()
    };
    let bands = |r: &[bool]| r.windows(2).filter(|w| !w[0] && w[1]).count() + usize::from(r[0]);
    assert_eq!(bands(&rows("Urząd")), 1);
    assert_eq!(bands(&rows("622@Urząd")), 2);
    assert_eq!(bands(&rows("622@Sosnowiec@Urząd")), 3);
}

    use super::*;

    fn independent_tree(id: i64, slot: usize) -> (i64, EditObject) {
        (id, EditObject {
            tile: (-18, 10), pos: DVec3::new(-5230.0, 3120.0, 0.29),
            xf: Mat4::from_scale(glam::Vec3::new(12.0, 12.0, 15.0)), key: id,
            instances: vec![slot], sco: PathBuf::from("Sceneryobjects/Krummenaab/Tree_Middle02.sco"),
            strings: vec!["Tree_middle02.dds".into(), "15".into(), "0.8".into()],
        })
    }

    #[test]
    fn scripted_object_pose_keeps_editor_move_turn_delete_and_undo() {
        let pos = DVec3::new(12.0, 34.0, 2.0);
        let base = Mat4::from_rotation_z(0.2);
        let animation = Mat4::from_rotation_z(1.1);
        let moved = ObjectEdit { moved: DVec3::new(3.0, -2.0, 0.5), turned: 20.0, deleted: false };
        let expected = Mat4::from_rotation_z(-20.0_f32.to_radians()) * base * animation;
        for _ in 0..40 { // repeated script frames must not put the original pose back
            let (at, rotation) = edited_object_pose(pos, base, moved);
            assert_eq!(at, pos + moved.moved);
            assert_eq!(rotation * animation, expected);
        }
        let (gone, _) = edited_object_pose(pos, base, ObjectEdit { deleted: true, ..moved });
        assert_eq!(gone, pos + moved.moved - DVec3::Z * 10_000.0);
        assert_eq!(edited_object_pose(pos, base, ObjectEdit::default()), (pos, base));
    }

    #[test]
    fn editor_rebuild_restores_each_trees_own_script_values_and_first_frame_pose() {
        let mut program = omsi_script::Program::default();
        let rotation = program.declare_var("random_rotation") as usize;
        program.declare_str_var("leaf_texture");
        let program = Arc::new(program);
        let sco = Path::new("Sceneryobjects/Krummenaab5/tree_medium_03.sco");
        let labels = vec!["leaf.dds".to_string()];
        let make = || {
            let mut inst = omsi_sim::scenery::SceneryInstance::new(
                program.clone(), &[], omsi_sim::SimClock::default(), &labels);
            inst.mesh_transforms = vec![Mat4::IDENTITY];
            inst.mesh_visible = vec![true];
            inst
        };
        let mut first = make(); first.state.vars[rotation] = 37.0;
        first.mesh_transforms[0] = Mat4::from_rotation_z(37.0_f32.to_radians());
        let mut second = make(); second.state.vars[rotation] = 121.0;
        second.mesh_transforms[0] = Mat4::from_rotation_z(121.0_f32.to_radians());
        second.mesh_visible[0] = false;
        let saved_first = EditorScriptState::capture(sco, &labels, &first);
        let saved_second = EditorScriptState::capture(sco, &labels, &second);
        for _ in 0..40 {
            let mut rebuilt_first = make(); let mut rebuilt_second = make();
            assert!(saved_first.restore(sco, &labels, &mut rebuilt_first));
            assert!(saved_second.restore(sco, &labels, &mut rebuilt_second));
            assert_eq!(rebuilt_first.state.vars, first.state.vars);
            assert_eq!(rebuilt_second.state.vars, second.state.vars);
            assert_eq!(rebuilt_first.state.str_vars, first.state.str_vars);
            assert_eq!(rebuilt_first.mesh_transforms, first.mesh_transforms);
            assert_eq!(rebuilt_second.mesh_transforms, second.mesh_transforms);
            assert_eq!(rebuilt_second.mesh_visible, second.mesh_visible);
        }
        assert!(!saved_first.restore(Path::new("other.sco"), &labels, &mut make()));
        assert!(!saved_first.restore(sco, &["changed.dds".into()], &mut make()));
        let mut incompatible = make(); incompatible.mesh_transforms.clear();
        assert!(!saved_first.restore(sco, &labels, &mut incompatible));
        let mut other_program = omsi_script::Program::default();
        other_program.declare_var("different_variable"); other_program.declare_str_var("leaf_texture");
        let mut incompatible = make(); incompatible.program = Arc::new(other_program);
        assert!(!saved_first.restore(sco, &labels, &mut incompatible));
    }

    #[test]
    fn repeated_spline_rebuilds_keep_tree_slots_and_transfer_ownership_once() {
        let mut tile = TileGpu { instances: vec![3, 7], trees: vec!["leaf-material".into()],
            editor_trees: vec![independent_tree(9858784, 7)], ..Default::default() };
        for step in 0..40 {
            let temporary = 100 + step;
            let ground_slot = 1000 + step;
            let mut replacement = TileGpu { instances: vec![ground_slot, temporary], trees: vec!["leaf-material".into()],
                editor_trees: vec![independent_tree(9858784, temporary)], ..Default::default() };
            let pairs = matching_editor_trees(&replacement, &tile);
            assert_eq!(pairs, vec![(0, 0)]);
            assert_eq!(transfer_editor_tree(&mut replacement, &mut tile, pairs[0]), Some((temporary, 7)));
            assert_eq!(transfer_editor_tree(&mut replacement, &mut tile, pairs[0]), None);
            assert!(!tile.instances.contains(&7), "old tile must not free the kept tree");
            assert_eq!(replacement.instances, vec![ground_slot, 7]);
            assert_eq!(replacement.editor_trees[0].1.instances, vec![7]);
            assert_eq!(tile.trees, replacement.trees, "each tile keeps its shared material reference until release");
            tile = replacement;
        }
    }

    #[test]
    fn changed_or_ambiguous_trees_are_not_kept_during_spline_rebuilds() {
        let old = TileGpu { instances: vec![7], editor_trees: vec![independent_tree(1, 7)], ..Default::default() };
        let make = || TileGpu { instances: vec![8], editor_trees: vec![independent_tree(1, 8)], ..Default::default() };
        for change in 0..7 {
            let mut current = make();
            let (id, object) = &mut current.editor_trees[0];
            match change {
                0 => *id = 2,
                1 => object.tile = (-17, 10),
                2 => object.pos.z += 0.1, // terrain editing must update tree height
                3 => object.xf = Mat4::from_rotation_z(0.5) * object.xf,
                4 => object.sco = PathBuf::from("Sceneryobjects/Other/tree.sco"),
                5 => object.strings[0] = "fall.dds".into(),
                _ => object.key = 2,
            }
            assert!(matching_editor_trees(&current, &old).is_empty(), "change {change}");
        }
        let mut duplicate = make();
        duplicate.editor_trees.push(independent_tree(1, 9)); duplicate.instances.push(9);
        assert!(matching_editor_trees(&duplicate, &old).is_empty());
        assert!(matching_editor_trees(&old, &duplicate).is_empty());
        let mut missing = make(); missing.instances.clear();
        assert!(matching_editor_trees(&missing, &old).is_empty());
        let row_or_helper = TileGpu { instances: vec![8], ..Default::default() };
        assert!(matching_editor_trees(&row_or_helper, &old).is_empty());
    }

    #[test]
    fn matching_a_cancelled_replacement_does_not_take_tree_ownership() {
        let old = TileGpu { instances: vec![7], editor_trees: vec![independent_tree(1, 7)], ..Default::default() };
        let replacement = TileGpu { instances: vec![8], editor_trees: vec![independent_tree(1, 8)], ..Default::default() };
        assert_eq!(matching_editor_trees(&replacement, &old), vec![(0, 0)]);
        drop(replacement); // abandoned upload frees only its own temporary slot
        assert_eq!(old.instances, vec![7]);
        assert_eq!(old.editor_trees[0].1.instances, vec![7]);
    }

    #[test]
    fn tree_cards_have_no_coplanar_duplicates_and_are_pickable_from_both_sides() {
        let mesh = tree_quad_mesh();
        assert!(!mesh.one_sided);
        assert_eq!(mesh.positions.len(), 42);
        assert_eq!(mesh.indices.len(), 144);
        let mut triangles = hashbrown::HashSet::new();
        for t in mesh.indices.chunks_exact(3) {
            let mut vertices = [t[0], t[1], t[2]];
            vertices.sort_unstable();
            assert!(triangles.insert(vertices), "duplicate coplanar tree triangle");
        }
        for (eye, direction) in [
            (glam::Vec3::new(0.1, -2.0, 0.5), glam::Vec3::Y),
            (glam::Vec3::new(0.1, 2.0, 0.5), glam::Vec3::NEG_Y),
        ] {
            assert!(omsi_geometry::ray_mesh(eye, direction, &mesh, &Mat4::IDENTITY).is_some());
        }
    }

    #[test]
    fn same_named_tree_textures_from_different_packs_do_not_share_materials() {
        let a = Path::new("Sceneryobjects/PackA/Texture/Tree_middle03.dds");
        let b = Path::new("Sceneryobjects/PackB/Texture/Tree_middle03.dds");
        let name = "Tree_middle03.dds";
        let ad = vec![a.parent().unwrap().to_path_buf()];
        let bd = vec![b.parent().unwrap().to_path_buf()];
        let ak = resolved_tree_material_key(name, &ad, Some(a));
        assert_ne!(ak, resolved_tree_material_key(name, &bd, Some(b)));
        assert_eq!(ak, resolved_tree_material_key("TREE_MIDDLE03.DDS", &bd, Some(a)));
        assert_ne!(resolved_tree_material_key(name, &ad, None), resolved_tree_material_key(name, &bd, None));
    }

    #[test]
    fn deleting_ground_object_removes_only_its_cut_and_restore_recuts_without_height_changes(){
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("junction-ground-delete-{stamp}"));
        let original=dir.join("original");let content=dir.join("content");let map=content.join("maps/Test");
        std::fs::create_dir_all(&original).unwrap();std::fs::create_dir_all(&map).unwrap();
        let sco=crate::junction_builder::export(&crate::junction_builder::Project::default(),&original,&content,&original).unwrap();
        // Explicit cutter makes the regression independent of the optional surface-cut setting.
        std::fs::write(&sco,"[absheight]\n[surface]\n[mesh]\nmodel/road.o3d\n[terrainhole]\nmodel/road.o3d\n").unwrap();
        let relative=sco.strip_prefix(&content).unwrap().to_string_lossy();
        let file=map.join("tile_0_0.map");
        std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n").unwrap();
        let record=|id,x|format!("[object]\n0\n{relative}\n{id}\n{x}\n100\n0\n0\n0\n0\n0\n");
        std::fs::write(&file,format!("[version]\n14\n[terrain]\n{}{}",record(7,80),record(8,200))).unwrap();
        let terrain_path=map.join("tile_0_0.map.terrain");let heights=Terrain::flat().to_bytes();std::fs::write(&terrain_path,&heights).unwrap();
        let world=World::open(&content,&map.join("global.cfg"),20000101).unwrap();let request=[(0,0,file)];
        for deleted in [false,true,false]{world.object_edits.lock().insert(7,ObjectEdit{deleted,..Default::default()});world.forget_staged(&[(0,0)]);
            let(prepared,_)=world.prepare_tiles(&request);assert_eq!(prepared.len(),1);
            let surfaces=world.surfaces.read();let surface=surfaces.get(&(0,0)).unwrap();
            assert_eq!(surface.cut_at(80.0,100.0,0.0,0.03),!deleted);
            assert!(surface.cut_at(200.0,100.0,0.0,0.03));
            assert_eq!(std::fs::read(&terrain_path).unwrap(),heights);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rebuilding_a_saved_tile_keeps_tree_headings_and_excludes_session_helpers() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("omsi-tree-rebuild-{}-{stamp}", std::process::id()));
        let map = dir.join("maps/Test"); let scenery = dir.join("Sceneryobjects");
        std::fs::create_dir_all(&map).unwrap(); std::fs::create_dir_all(&scenery).unwrap();
        std::fs::write(map.join("global.cfg"), "[map]\n0\n0\ntile_0_0.map\n").unwrap();
        std::fs::write(scenery.join("tree.sco"), "[tree]\nleaf.png\n6\n10\n0.3\n0.5\n").unwrap();
        let record = |id, heading| format!("[object]\n0\nSceneryobjects\\tree.sco\n{id}\n20\n40\n1.25\n{heading}\n0\n0\n3\nleaf.png\n8\n0.4\n");
        let file = map.join("tile_0_0.map");
        std::fs::write(&file, format!("[version]\n14\n[terrain]\n{}{}", record(7, 15), record(8, 40))).unwrap();
        std::fs::write(map.join("tile_0_0.map.terrain"), Terrain::flat().to_bytes()).unwrap();
        let world = World::open(&dir, &map.join("global.cfg"), 20000101).unwrap();
        let request = [(0, 0, file.clone())];
        let (first, _) = world.prepare_tiles(&request);
        assert_eq!(first.len(), 1); assert_eq!(first[0].trees.len(), 2);
        let headings: HashMap<_, _> = first[0].trees.iter().map(|tree| (tree.6.as_ref().unwrap().0, tree.5)).collect();
        // Simulate a saved overlay, including a helper's record. The tile is read anew
        // after a spline edit, while the editor still owns object 9 independently.
        world.retain_editor_helper(9);
        std::fs::write(&file, format!("[version]\n14\n[terrain]\n{}{}{}", record(7, 20), record(8, 45), record(9, 90))).unwrap();
        for _ in 0..3 {
            world.forget_staged(&[(0, 0)]);
            let (rebuilt, _) = world.prepare_tiles(&request);
            assert_eq!(rebuilt.len(), 1); assert_eq!(rebuilt[0].trees.len(), 2);
            for tree in &rebuilt[0].trees {
                let id = tree.6.as_ref().unwrap().0;
                assert_eq!(tree.5, headings[&id], "spline rebuild must not turn a tree");
                assert_eq!((tree.3, tree.4), (8.0, 3.2));
            }
        }
        // A new map session reads the authored headings and all saved objects normally.
        let reopened = World::open(&dir, &map.join("global.cfg"), 20000101).unwrap();
        let (fresh, _) = reopened.prepare_tiles(&request);
        assert_eq!(fresh[0].trees.len(), 3);
        assert!(fresh[0].trees.iter().any(|tree| tree.6.as_ref().unwrap().0 == 9));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn deleting_a_construction_light_removes_its_glow_and_maplight_and_undo_restores_both() {
        let origin = DVec3::new(100_000_000.0, 20.0, 3.0);
        let position = origin + DVec3::new(2.0, 0.0, 1.0);
        let corona = StaticCorona { corona: omsi_render::Corona {
            position, direction: glam::Vec3::X, brightness: 0.65,
            ..Default::default()
        }, switch: LightSwitch::Constant(0.65) };
        let point = omsi_render::PointLight { position, intensity: 0.7, ..Default::default() };
        let mut state = TileState::default();
        state.coronas = vec![corona.clone(), corona.clone(), corona];
        state.lights = vec![point.clone(), point.clone(), point];
        state.corona_owners = vec![Some((7, origin)), Some((8, origin)), None];
        state.light_owners = state.corona_owners.clone();
        let mut edits = HashMap::new();
        edits.insert(7, ObjectEdit { deleted: true, ..Default::default() });
        let (glows, points) = collect_editor_lights(&state, &edits);
        assert_eq!((glows.len(), points.len()), (2, 2));
        assert_eq!(state.coronas.len(), 3, "tile originals survive for undo/reloading");
        edits.insert(7, ObjectEdit { moved: DVec3::new(10.0, 20.0, 4.0), turned: 90.0, deleted: false });
        let (glows, points) = collect_editor_lights(&state, &edits);
        let expected = origin + DVec3::new(10.0, 18.0, 5.0);
        assert!((glows[0].corona.position - expected).length() < 1e-6);
        assert!((points[0].position - expected).length() < 1e-6);
        assert!((glows[0].corona.direction - glam::Vec3::NEG_Y).length() < 1e-6);
        assert_eq!(glows[0].corona.brightness, 0.65);
        assert_eq!(points[0].intensity, 0.7);
        assert_eq!(glows[1].corona.position, position, "the neighboring lamp is unchanged");
        let repeated = collect_editor_lights(&state, &edits);
        assert_eq!(repeated.0[0].corona.position, glows[0].corona.position, "no accumulated transforms");
        edits.remove(&7);
        let restored = collect_editor_lights(&state, &edits);
        assert_eq!((restored.0.len(), restored.1.len()), (3, 3));
        assert_eq!(restored.0[0].corona.position, position);
        assert_eq!(restored.0[0].corona.direction, glam::Vec3::X);
    }

    #[test]
    fn construction_glows_are_pickable_at_large_coordinates_without_selecting_other_lamps() {
        let eye = DVec3::new(100_000_000.0, 200_000_000.0, 3.0);
        let point = eye + DVec3::new(0.1, 20.0, 0.0);
        assert_eq!(editor_light_hit(eye, glam::Vec3::Y, point), Some(20.0));
        assert!(editor_light_hit(eye, glam::Vec3::Y, point + DVec3::X).is_none());
        assert!(editor_light_hit(eye, glam::Vec3::NEG_Y, point).is_none());
        assert!(editor_light_hit(eye, glam::Vec3::ZERO, point).is_none());
    }

    #[test]
    fn tree_editor_geometry_keeps_copied_size_and_is_pickable_on_its_crown() {
        let sco=SceneryObject {tree:Some(("leaf.png".into(),6.0,10.0,0.3,0.5)),..Default::default()};
        let strings=vec!["leaf2.png".into(),"8".into(),"0.4".into()];
        let shape=editor_helper_shape(&sco,&strings,[0.0;2]);
        assert!((shape.transform_vector3(glam::Vec3::X).length()-3.2).abs()<1e-5);
        assert_eq!(shape.transform_vector3(glam::Vec3::Z).length(),8.0);
        let pos=DVec3::new(300.0,30.0,40.0);let mesh=tree_quad_mesh();
        let hit=editor_mesh_hit(&mesh,(glam::Vec3::new(0.0,0.0,0.5),0.8),pos+DVec3::new(0.0,20.0,4.0),glam::Vec3::NEG_Y,pos,shape).unwrap();
        assert!((hit-20.0).abs()<1e-4);
        assert!(editor_mesh_hit(&mesh,(glam::Vec3::new(0.0,0.0,0.5),0.8),pos+DVec3::new(20.0,20.0,4.0),glam::Vec3::NEG_Y,pos,shape).is_none());
    }

    #[test]
    fn junction_surface_is_pickable_far_from_its_pivot_after_moving_and_turning() {
        let mesh = MeshData { positions: vec![glam::Vec3::new(20.0, -5.0, 0.0), glam::Vec3::new(70.0, -5.0, 0.0),
            glam::Vec3::new(70.0, 5.0, 0.0), glam::Vec3::new(20.0, 5.0, 0.0)],
            indices: vec![0, 1, 2, 0, 2, 3], ..Default::default() };
        let bounds = omsi_geometry::bounding_sphere(&mesh.positions);
        let pos = DVec3::new(100_000_000.0, 200_000_000.0, 2.0);
        let xf = Mat4::from_rotation_z(-std::f32::consts::FRAC_PI_2);
        let point = pos + xf.transform_point3(glam::Vec3::new(50.0, 2.0, 0.0)).as_dvec3();
        let eye = point + DVec3::Z * 30.0;
        let hit = editor_mesh_hit(&mesh, bounds, eye, glam::Vec3::NEG_Z, pos, xf).unwrap();
        assert!((hit - 30.0).abs() < 1e-4);
        assert!((point - pos).truncate().length() > 40.0);
        assert!(editor_mesh_hit(&mesh, bounds, pos + DVec3::Z * 30.0, glam::Vec3::NEG_Z, pos, xf).is_none());
        assert!(editor_mesh_hit(&mesh, bounds, eye, glam::Vec3::NEG_Z, pos + DVec3::X * 100.0, xf).is_none());
        assert!(editor_mesh_hit(&mesh, bounds, point + DVec3::Z * 500.0, glam::Vec3::NEG_Z, pos, xf).is_none());
    }

    struct SceneryFixture(PathBuf);

    impl SceneryFixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!("openomsi-editor-object-{}-{}-{}",
                std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let path = self.0.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            path
        }
    }

    impl Drop for SceneryFixture {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn catalog_absolute_object_and_model_paths_are_not_prefixed_with_the_game_root() {
        let fixture = SceneryFixture::new();
        let root = fixture.0.join("game");
        let sco_path = fixture.write("editor-content/Sceneryobjects/Junctions/tee.sco", "[model]\nModel\\junction.cfg\n");
        fixture.write("editor-content/Sceneryobjects/Junctions/Model/junction.cfg", "[mesh]\nroad.o3d\n");
        assert_eq!(scenery_asset_path(&root, &sco_path.to_string_lossy()), sco_path);
        let (sco, sco_dir, model, model_dir) = scenery_definition(&root, &sco_path.to_string_lossy()).unwrap();
        assert_eq!(sco.model_file.as_deref(), Some(r"Model\junction.cfg"));
        assert_eq!(sco_dir, sco_path.parent().unwrap());
        assert_eq!(model_dir, sco_path.parent().unwrap().join("Model"));
        assert_eq!(model.meshes[0].file, "road.o3d");
    }

    #[test]
    fn map_relative_windows_paths_and_inline_models_still_load() {
        let fixture = SceneryFixture::new();
        let sco_path = fixture.write("Sceneryobjects/Junctions/tee.sco", "[mesh]\nroad.o3d\n");
        let relative = r"Sceneryobjects\Junctions\tee.sco";
        assert_eq!(scenery_asset_path(&fixture.0, relative), sco_path);
        let (sco, _, model, model_dir) = scenery_definition(&fixture.0, relative).unwrap();
        assert!(sco.model_file.is_none());
        assert_eq!(model_dir, sco_path.parent().unwrap());
        assert_eq!(model.meshes[0].file, "road.o3d");
    }

    #[test]
    fn missing_object_and_external_model_errors_identify_the_file() {
        let fixture = SceneryFixture::new();
        let missing = "Sceneryobjects/Junctions/absent.sco";
        let Err(error) = scenery_definition(&fixture.0, missing) else { panic!("missing .sco loaded") };
        assert!(error.contains("SCO file missing") && error.contains("absent.sco"), "{error}");
        let sco_path = fixture.write("Sceneryobjects/Junctions/tee.sco", "[model]\nModel\\missing.cfg\n");
        let Err(error) = scenery_definition(&fixture.0, &sco_path.to_string_lossy()) else { panic!("missing model loaded") };
        assert!(error.contains("Model definition missing") && error.contains("missing.cfg"), "{error}");
        assert!(error.contains(&sco_path.parent().unwrap().to_string_lossy().to_string()), "{error}");
    }

    #[test]
    fn far_offset_forest_backdrops_keep_their_owner_tiles_visibility() {
    let rectangle = |x: f32, half_width: f32| MeshData {
        positions: vec![
            glam::Vec3::new(x, -half_width, -80.0),
            glam::Vec3::new(x, half_width, -80.0),
            glam::Vec3::new(x, half_width, 127.0),
            glam::Vec3::new(x, -half_width, 127.0),
        ],
        ..Default::default()
    };
    let loaded = 900.0;
    let forest = rectangle(-1109.0, 693.0);
    for heading in [0.0, 45.0, 225.0] {
        let xf = object_rotation([heading, 0.0, 0.0]);
        assert!(stand_in_mesh(&forest, &[], &xf, DVec3::new(6242.0, 3348.0, 70.0), loaded),
            "the distant forest at heading {heading} must not cover a road outside its owner tiles");
    }
    // Ordinary large geometry beside its origin still uses the full view distance.
    assert!(!stand_in_mesh(&rectangle(0.0, 693.0), &[], &Mat4::IDENTITY, DVec3::ZERO, loaded));
    // A small offset part is not enough to classify an object as a far backdrop.
    assert!(!stand_in_mesh(&rectangle(-1109.0, 5.0), &[], &Mat4::IDENTITY, DVec3::ZERO, loaded));
    assert!(!stand_in_mesh(&forest, &[], &Mat4::from_scale(glam::Vec3::splat(0.2)), DVec3::ZERO, loaded));
    // Keep the existing whole-city stand-in rule, even for a centred model.
    assert!(stand_in_mesh(&rectangle(0.0, 2000.0), &[], &Mat4::IDENTITY, DVec3::ZERO, loaded));
}

#[test]
fn a_mirror_keeps_the_glass_that_uses_most_of_the_picture() {
    let glass = |uv_area: f32, x: f32| MirrorGlass { centre: glam::Vec3::new(x, 0.0, 0.0), du: glam::Vec3::X, dv: glam::Vec3::Z, uv_area };
    let big = larger_glass(None, glass(0.4, 1.0)).unwrap();
    // a smaller mesh for the same mirror, loaded later, does not take its place
    assert_eq!(larger_glass(Some(big), glass(0.001, 2.0)).unwrap().centre.x, 1.0);
    // a larger one does
    assert_eq!(larger_glass(Some(big), glass(0.9, 3.0)).unwrap().centre.x, 3.0);
}

/// A route arrow's Cyrillic street name with the stock Latin-only "test" font: drawn
/// with the interface font (it was an empty texture); a Latin one keeps the .oft.
#[test]
fn a_helper_text_the_font_cannot_draw_comes_from_the_interface_font() {
    use omsi_content::font::{Font, FontAtlas, FontChar};
    // a Latin font with its umlauts (`Ä` is `Д` in code page 1251, so one Cyrillic
    // letter alone is "in" the font)
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzÄÖÜäöüß"
        .chars()
        .enumerate()
        .map(|(k, ch)| FontChar { ch, x0: k as i32 * 4, x1: k as i32 * 4 + 3, y: 0 })
        .collect();
    let font = Font { path: PathBuf::new(), name: "test".into(), bitmap: String::new(), alpha: String::new(), height: 27, gap: 1, chars };
    let (aw, ah) = (256u32, 32u32);
    let atlas = FontAtlas::new(font, aw, ah, vec![255; (aw * ah * 4) as usize], vec![255; (aw * ah * 4) as usize]);
    let tt = omsi_model::TextTexture { variable: "0".into(), font: "test".into(), width: 128, height: 128, full_color: false, color: [255.0, 0.0, 0.0], orientation: 0, grid: 1 };
    assert!(helper_text_image(&tt, Some(&atlas), "Bauernhof").is_none());
    assert!(helper_text_image(&tt, Some(&atlas), "  ").is_none());
    for text in ["Улица Ленина", "Булевар ослобођења", "Δ"] {
        let img = helper_text_image(&tt, Some(&atlas), text).unwrap_or_else(|| panic!("{text}: drawn with the .oft"));
        assert_eq!((img.width, img.height), (128, 128));
        let ink: Vec<usize> = (0..128 * 128).filter(|&p| img.rgba[p * 4 + 3] > 128).collect();
        assert!(ink.len() > 40, "{text}: {} pixels", ink.len());
        assert!(ink.iter().all(|&p| img.rgba[p * 4..p * 4 + 3] == [255, 0, 0]), "{text}: in the texture's colour");
        // centred, and a long name narrowed into the texture
        let rows: Vec<usize> = ink.iter().map(|p| p / 128).collect();
        let (top, bottom) = (*rows.iter().min().unwrap(), *rows.iter().max().unwrap());
        assert!(top > 40 && bottom < 88, "{text}: rows {top}..{bottom}");
    }
    // no font at all (missing from the installation): still readable
    assert!(helper_text_image(&tt, None, "Bauernhof").is_some());
}

#[test]
fn nightlight_follows_the_objects_darkness_threshold() {
    let day = DayKind { workday: true, ..Default::default() };
    let plain = InUse::new(0, 7);
    assert!(plain.lit(12.0 * 3600.0, day, 0.5));
    assert!(!plain.lit(12.0 * 3600.0, day, 0.65));
    let home = InUse::new(2, 7);
    assert!((0.3..=0.75).contains(&home.threshold));
    assert!(!home.lit(3.0 * 3600.0, day, 0.0));
}

#[test]
fn vehicle_freetex_retries_paths_below_texture_component() {
    let root = std::env::temp_dir().join("openomsi-freetex-path-test");
    let vehicle_texture = root.join("Vehicles/TestBus/Texture");
    let wanted = vehicle_texture.join("mb_pmon/alerta_FalhaCambio.bmp");
    std::fs::create_dir_all(wanted.parent().unwrap()).unwrap();
    std::fs::write(&wanted, b"x").unwrap();
    let dirs = [vehicle_texture.as_path()];
    let found = find_vehicle_freetex(
        r"..\Texture\mb_pmon\alerta_FalhaCambio.bmp",
        &dirs,
    );
    assert_eq!(found.as_deref(), Some(wanted.as_path()));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_child_naming_a_crossing_light_reads_that_light() {
    // (RefreshAmpelParenting hands any child of a running crossing whose first string
    // is a light index that light's phase, lamp or not, #922)
    let crossings: hashbrown::HashSet<i64> = [7].into_iter().collect();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(light_child_of(&crossings, Some(7), &s(&["2", "x"])), Some((7, 2)));
    assert_eq!(light_child_of(&crossings, Some(7), &s(&[" 0 "])), Some((7, 0)));
    assert_eq!(light_child_of(&crossings, Some(8), &s(&["2"])), None, "not a crossing with lights");
    assert_eq!(light_child_of(&crossings, None, &s(&["2"])), None);
    assert_eq!(light_child_of(&crossings, Some(7), &s(&["Hbf"])), None);
    assert_eq!(light_child_of(&crossings, Some(7), &s(&["-1"])), None);
}

#[test]
fn traffic_light_program_requires_a_placed_signal() {
    let sco = SceneryObject::parse(&omsi_cfg::CfgFile::from_str("junction.sco", "[traffic_lights_group]\n72\n[traffic_light]\nMain\n[phase]\n0\n10\n[phase]\n6\n62\n"));
    assert!(!traffic_light_program_enabled(&sco, false));
    assert!(traffic_light_program_enabled(&sco, true));
    let gate = SceneryObject { is_traffic_light: true, ..sco };
    assert!(traffic_light_program_enabled(&gate, false));
    assert!(!traffic_light_program_enabled(&SceneryObject::default(), true));
}

fn freetex_test_look() -> Look {
    Look {
        alpha: AlphaMode::Opaque,
        color: [1.0; 4],
        emissive: [0.0; 3],
        unlit: false,
        diffuse: None,
        transmap: None,
        night: None,
        lightmap: None,
        envmap: None,
        extra: MaterialExtra::default(),
        dyn_tex: DynTex::default(),
    }
}

/// An LED panel's light map is one white pixel; a flipdot's is a picture with dark
/// parts (the Krueger's `vmatrix_leer_LM.bmp`), and does not make an LED panel (#413).
#[test]
fn a_clear_texture_is_an_invisible_cover() {
    assert!(texture_is_clear(&vec![0u8; 64]));
    assert!(texture_is_clear(&vec![5u8; 64]));
    let mut pane = vec![0u8; 64];
    pane[10] = 62;
    assert!(!texture_is_clear(&pane));
    assert!(!texture_is_clear(&[]));
}

#[test]
fn a_transmapped_slot_is_see_through_by_its_transmap_not_its_reflection_mask() {
    // the stock Golf 2: diffuse alpha 0 (reflection mask), transmap opaque (#928, #932)
    assert_eq!(coverage_texture(Some("Golf2_main1_T.tga"), "Golf2_main1.tga"), "Golf2_main1_T.tga");
    assert_eq!(coverage_texture(None, "glass.tga"), "glass.tga");
    assert_eq!(coverage_texture(Some("  "), "glass.tga"), "glass.tga");
    assert_eq!(coverage_texture(Some("\\S:2"), "led.tga"), "led.tga");
}

#[test]
fn a_lamps_lenses_follow_its_alphascale_and_light_map_variables() {
    // three lenses on one mesh, each faded by its colour's variable and lit by its
    // light map (Cheongsan's signals, #826); a fourth slot switched by nothing
    let slots = LampSlots {
        count: 4,
        alpha: vec![(0, "Red".into()), (1, "Yellow".into()), (2, "Green".into())],
        light: vec![(0, "Red".into()), (1, "Yellow".into()), (2, "Green".into()), (3, "NoSuchVar".into())],
    };
    let state = |v: &str| standard_traffic_lamp(v, true, false, false, false);
    let (alpha, light) = slots.values(&state);
    assert_eq!(alpha, vec![1.0, 0.0, 0.0, 1.0]);
    assert_eq!(light, vec![1.0, 0.0, 0.0, 1.0]);
    let state = |v: &str| standard_traffic_lamp(v, false, false, true, false);
    assert_eq!(slots.values(&state), (vec![0.0, 0.0, 1.0, 1.0], vec![0.0, 0.0, 1.0, 1.0]));
    // a light map without a variable is always on
    let plain = LampSlots { count: 1, alpha: vec![], light: vec![(0, String::new())] };
    assert_eq!(plain.values(&|_| Some(0.0)).1, vec![1.0]);
}

/// A season's snow textures are told by their folder, whatever its case (#879).
#[test]
fn snow_pictures_are_the_winter_snow_folders() {
    assert!(is_snow_picture(Path::new("/omsi/Texture/WinterSnow/gras.bmp")));
    assert!(is_snow_picture(Path::new("/omsi/Sceneryobjects/Buildings_RW1HH/texture/Wintersnow/wall.jpg")));
    assert!(!is_snow_picture(Path::new("/omsi/Texture/Winter/gras.bmp")));
    assert!(!is_snow_picture(Path::new("/omsi/Texture/WinterSnow_gras.bmp")));
}

#[test]
fn only_a_white_light_map_makes_an_led_panel() {
    assert!(is_white_lightmap(&[255, 255, 255, 255]));
    assert!(is_white_lightmap(&[250, 248, 255, 0, 255, 255, 255, 255]));
    assert!(!is_white_lightmap(&[255, 255, 255, 255, 127, 127, 127, 255]));
    assert!(!is_white_lightmap(&[0, 0, 0, 255]));
    assert!(!is_white_lightmap(&[]));
}

#[test]
#[ignore = "requires the installed SOR NB content in OMSI_TEST_CONTENT"]
fn installed_sor_ois_retains_powered_freetex() {
    let root = PathBuf::from(
        omsi_cfg::flags::OMSI_TEST_CONTENT.live_os()
            .expect("set OMSI_TEST_CONTENT to the OMSI content root"),
    );
    let model =
        omsi_model::Model::load(&root.join("Vehicles/SOR NB/model/1_2011.cfg")).unwrap();
    let definitions: Vec<_> = model
        .meshes
        .iter()
        .flat_map(|mesh| {
            let refs: Vec<_> = mesh.materials.iter().collect();
            free_texture_defs(&refs)
        })
        .filter(|(_, _, var)| var == "mypoldisplej")
        .collect();
    assert!(!definitions.is_empty());
    assert!(
        definitions
            .iter()
            .any(|(item, key, _)| *item && key.eq_ignore_ascii_case("cerna.bmp")),
        "{definitions:?}"
    );
    println!("SOR NB OIS: {definitions:?}");
}

#[test]
fn powered_terminal_freetex_is_kept_and_replaces_its_black_nightmap() {
    // The vehicle's OIS declares the free texture in the powered item, not in
    // the base [matl]. The black key is also used as its self-lit night map.
    let model = omsi_model::Model::parse(&omsi_cfg::CfgFile::from_str(
        "model.cfg",
        concat!(
            "[mesh]\nterminal.o3d\n[matl]\nblack.bmp\n0\n",
            "[matl_change]\nblack.bmp\n0\npower\n[matl_item]\n",
            "[matl_nightmap]\nblack.bmp\n[matl_freetex]\nblack.bmp\nscreen\n",
        ),
    ));
    let defs: Vec<&MaterialDef> = model.meshes[0].materials.iter().collect();
    assert_eq!(
        free_texture_defs(&defs),
        vec![(true, "black.bmp".into(), "screen".into())]
    );
    let base = freetex_test_look();
    let mut powered = base.clone();
    powered.night = Some(10);
    let spec = SlotSpec {
        base,
        item: Some(powered),
        more: Vec::new(),
    };
    let changed = spec.with_freetex(Some(10), 20, true, true);
    assert_eq!(changed.base.diffuse, None); // unpowered remains black
    assert_eq!(changed.item.as_ref().unwrap().diffuse, Some(20));
    assert_eq!(changed.item.as_ref().unwrap().night, Some(20));
    assert_eq!(spec.item.as_ref().unwrap().night, Some(10)); // reusable template
}

#[test]
fn freetex_preserves_other_stages_and_per_vehicle_script_textures() {
    let mut base = freetex_test_look();
    base.night = Some(10);
    base.lightmap = Some(11);
    base.transmap = Some((10, true));
    base.envmap = Some((12, 0.5));
    let mut item = base.clone();
    item.diffuse = Some(99); // a script texture is not the file being replaced
    let spec = SlotSpec {
        base,
        item: Some(item),
        more: Vec::new(),
    };
    let changed = spec.with_freetex(Some(10), 20, true, false);
    assert_eq!(changed.base.diffuse, Some(20));
    assert_eq!(changed.base.night, Some(20));
    assert_eq!(changed.base.lightmap, Some(11));
    assert_eq!(changed.base.transmap, Some((20, true)));
    assert_eq!(changed.base.envmap, Some((12, 0.5)));
    assert_eq!(changed.item.as_ref().unwrap().diffuse, Some(99));
    let missing_key = spec.with_freetex(None, 21, true, true);
    assert_eq!(missing_key.item.as_ref().unwrap().night, Some(10));
    assert_eq!(missing_key.item.as_ref().unwrap().diffuse, Some(99));
}

#[test]
fn spline_batches_keep_materials_cells_shadows_and_long_segments_separate() {
    use omsi_scenery::sli::SplineTexture;
    let def = |file: &str| Spline {
        textures: vec![SplineTexture { file: file.into(), ..Default::default() }],
        ..Default::default()
    };
    let ty = Arc::new(SplineType { def: def("curb.dds"), dir: PathBuf::new(), surf: Vec::new(), surface: Vec::new() });
    let other = Arc::new(SplineType { def: def("other.dds"), dir: PathBuf::new(), surf: Vec::new(), surface: Vec::new() });
    let other_dir = Arc::new(SplineType { def: def("curb.dds"), dir: PathBuf::from("another_pack"), surf: Vec::new(), surface: Vec::new() });
    let mut tested = def("curb.dds");
    tested.textures[0].alpha = 1;
    let tested = Arc::new(SplineType { def: tested, dir: PathBuf::new(), surf: Vec::new(), surface: Vec::new() });
    let mut blended = def("curb.dds");
    blended.textures[0].alpha = 2;
    let blended = Arc::new(SplineType { def: blended, dir: PathBuf::new(), surf: Vec::new(), surface: Vec::new() });
    let mut compatible = def("curb.dds");
    compatible.path = PathBuf::from("another_profile.sli");
    compatible.textures.push(SplineTexture { file: "unused-grass.dds".into(), ..Default::default() });
    let compatible = Arc::new(SplineType { def: compatible, dir: PathBuf::new(), surf: Vec::new(), surface: Vec::new() });
    let mesh = |x: f32, length: f32| Arc::new(MeshData {
        positions: vec![glam::Vec3::new(x, 0.0, 0.0), glam::Vec3::new(x + length, 0.0, 0.0), glam::Vec3::new(x, 1.0, 0.0)],
        normals: vec![glam::Vec3::Z; 3],
        uvs: vec![glam::Vec2::ZERO; 3],
        indices: vec![0, 1, 2],
        ranges: vec![(0, 3, 0)],
        one_sided: true,
    });
    let batched = batch_static_splines(vec![
        (mesh(1.0, 2.0), ty.clone(), false, DVec3::ZERO),
        (mesh(5.0, 2.0), ty.clone(), false, DVec3::ZERO),
        (mesh(9.0, 2.0), compatible, false, DVec3::ZERO),
        (mesh(49.0, 2.0), ty.clone(), false, DVec3::ZERO),
        (mesh(1.0, 2.0), other, false, DVec3::ZERO),
        (mesh(1.0, 2.0), other_dir, false, DVec3::ZERO),
        (mesh(1.0, 2.0), tested, false, DVec3::ZERO),
        (mesh(1.0, 2.0), ty.clone(), true, DVec3::ZERO),
        (mesh(1.0, 100.0), ty.clone(), false, DVec3::ZERO),
        (mesh(1.0, 100.0), ty, false, DVec3::ZERO),
        (mesh(1.0, 2.0), blended.clone(), false, DVec3::new(1.0, 2.0, 3.0)),
        (mesh(5.0, 2.0), blended, false, DVec3::new(4.0, 5.0, 6.0)),
    ]);
    assert_eq!(batched.len(), 10);
    assert_eq!(batched[8].0.indices.len(), 3);
    assert_eq!(batched[9].0.indices.len(), 3);
    assert_eq!(batched[8].3, DVec3::new(1.0, 2.0, 3.0));
    assert_eq!(batched[9].3, DVec3::new(4.0, 5.0, 6.0));
    assert_eq!(batched[0].0.indices.len(), 9);
    assert_eq!(batched[0].0.ranges, vec![(0, 9, 0)]);
    assert_eq!(batched.iter().filter(|b| b.2).count(), 1);
    assert_eq!(batched.iter().map(|b| b.0.indices.len()).sum::<usize>(), 36);
}

#[test]
fn ground_spline_batches_preserve_faces_and_uvs_with_local_bounds() {
    let mesh = |x: f32, z: f32, length: f32, one_sided: bool| Arc::new(MeshData {
        positions: vec![glam::Vec3::new(x, 0.0, z), glam::Vec3::new(x + length, 0.0, z), glam::Vec3::new(x, 1.0, z)],
        normals: vec![glam::Vec3::Z; 3],
        uvs: vec![glam::Vec2::new(x / 300.0, z / 300.0); 3],
        indices: vec![0, 1, 2],
        ranges: vec![(0, 3, 0)],
        one_sided,
    });
    let a = mesh(1.0, 0.0, 2.0, true);
    let b = mesh(5.0, 0.0, 2.0, true);
    let batched = batch_ground_splines(vec![
        a.clone(), b.clone(),
        mesh(49.0, 0.0, 2.0, true),
        mesh(1.0, 49.0, 2.0, true),
        mesh(1.0, 0.0, 2.0, false),
        mesh(1.0, 0.0, 100.0, true),
        mesh(1.0, 0.0, 100.0, true),
    ]);
    assert_eq!(batched.len(), 6);
    let combined = &batched[0];
    assert_eq!(combined.positions, [a.positions.clone(), b.positions.clone()].concat());
    assert_eq!(combined.normals, [a.normals.clone(), b.normals.clone()].concat());
    assert_eq!(combined.uvs, [a.uvs.clone(), b.uvs.clone()].concat());
    assert_eq!(combined.indices, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(combined.ranges, vec![(0, 6, 0)]);
    assert_eq!(batched.iter().map(|m| m.indices.len()).sum::<usize>(), 21);
}

/// A light map covers the 3x3 tiles around its own: a lamp's pool in the middle of the
/// picture is in the middle of the tile, one in a neighbour's third is left out, and the
/// tile's edges take the texels a third of the way in.
#[test]
fn a_light_map_is_laid_on_its_middle_third() {
    let n = 12usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) * 4;
            rgba[i] = (x * 20) as u8;
            rgba[i + 1] = (y * 20) as u8;
            rgba[i + 3] = 255;
        }
    }
    // a pool in the western neighbour, left out
    rgba[(6 * n + 1) * 4 + 2] = 255;
    let img = omsi_texture::Image { width: n as u32, height: n as u32, rgba, has_alpha: false };
    let own = own_tile_of_light_map(&img);
    assert_eq!((own.width, own.height), (n as u32, n as u32));
    let at = |x: usize, y: usize, c: usize| own.rgba[(y * n + x) * 4 + c] as f32;
    // output texel x samples x = 4 + (x + 0.5) / 3 - 0.5 of the source
    let expect = |x: usize| 20.0 * (4.0 + (x as f32 + 0.5) / 3.0 - 0.5);
    for x in [0, 5, 11] {
        assert!((at(x, 0, 0) - expect(x)).abs() <= 1.0, "column {x}: {} against {}", at(x, 0, 0), expect(x));
        assert!((at(0, x, 1) - expect(x)).abs() <= 1.0, "row {x}");
    }
    assert!((0..n * n).all(|i| own.rgba[i * 4 + 2] == 0));
}

/// A DLC's buildings grouped as "Thüringer Wald" are no plants; trees and hedges are
/// (#1775).
#[test]
fn a_forest_named_map_is_no_plant() {
    let sco = |text: &str| SceneryObject::parse(&omsi_cfg::CfgFile::from_str("haus.sco", text));
    assert!(vegetation_give_of(&sco("[groups]\n2\nThüringer Wald\nGebäude\n[mesh]\nhaus.o3d\n")).is_none());
    assert!(vegetation_give_of(&sco("[groups]\n2\nThüringer Wald\nBäume\n[mesh]\nbaum.o3d\n")).is_some());
    assert!(vegetation_give_of(&sco("[groups]\n1\nHedges\n[mesh]\nh.o3d\n")).is_some());
    // ("street" holds "tree": OMSI's street objects are no plants)
    assert!(vegetation_give_of(&sco("[groups]\n2\nGerman Street Side\nLights\n[mesh]\nlamp.o3d\n")).is_none());
    assert!(vegetation_give_of(&sco("[groups]\n2\nTrees LQ\nShrubbery\n[mesh]\ns.o3d\n")).is_some());
    assert!(vegetation_give_of(&sco("[groups]\n1\nStreet Side\n[groups]\n1\nPlants\n[mesh]\nt.o3d\n")).is_some());
}

/// A bus bay's lines made as a plain object (NCCR's `Parkbox(bus).sco`: a flat mesh 5 mm
/// over a road at 10 cm) are paint and drawn over the road as the markings are (#1009);
/// a kerb, a sign, a pole, a box under the road, a typed marking and an empty object
/// are not.
#[test]
fn a_flat_plain_object_is_paint_on_the_road() {
    let sco = |text: &str| SceneryObject::parse(&omsi_cfg::CfgFile::from_str("x.sco", text));
    let mesh = |zs: &[f32]| {
        let positions: Vec<glam::Vec3> = zs.iter().enumerate().map(|(i, z)| glam::Vec3::new(i as f32, (i % 2) as f32 * 5.0, *z)).collect();
        let n = positions.len();
        (
            MeshData { positions, normals: vec![glam::Vec3::Z; n], uvs: vec![glam::Vec2::ZERO; n], ranges: vec![(0, 3, 0)], indices: vec![0, 1, 2], one_sided: true },
            Vec::new(),
            Vec::new(),
        )
    };
    let plain = sco("[mesh]\nParkbox.o3d\n");
    assert!(paint_at_foot(&plain, &[mesh(&[0.105, 0.105, 0.105, 0.105])]));
    // a slightly sunk plate, and lines raised a little over it
    assert!(paint_at_foot(&plain, &[mesh(&[-0.04, -0.03, -0.04]), mesh(&[0.0, 0.01, 0.0])]));
    assert!(paint_at_foot(&plain, &[mesh(&[0.2, 0.25, 0.22])]));
    // a kerb stone 30 cm high, a line plate standing upright (Busstop_Lineplate_Bridge,
    // -1 to 15 cm), a pole, a box under the road
    assert!(!paint_at_foot(&plain, &[mesh(&[0.0, 0.3, 0.1])]));
    assert!(!paint_at_foot(&plain, &[mesh(&[-0.01, 0.15, 0.07])]));
    assert!(!paint_at_foot(&plain, &[mesh(&[0.1, 0.1, 0.1]), mesh(&[0.0, 2.5, 0.0])]));
    assert!(!paint_at_foot(&plain, &[mesh(&[-0.3, -0.1, -0.2])]));
    assert!(!paint_at_foot(&plain, &[]));
    // the typed ones are drawn over the roads as surfaces already
    assert!(!paint_at_foot(&sco("[rendertype]\non_surface\n[mesh]\narrow.o3d\n"), &[mesh(&[0.11, 0.11, 0.11])]));
    assert!(!paint_at_foot(&sco("[surface]\n[mesh]\nplate.o3d\n"), &[mesh(&[0.0, 0.0, 0.0])]));
}

#[test]
#[ignore = "requires a graphics adapter; checks road marking composition"]
fn painted_spline_stays_above_roads_and_below_raised_surfaces() {
    let def = omsi_scenery::sli::Spline::parse(&omsi_cfg::CfgFile::from_str(
        "synthetic.sli",
        "[texture]\nasphalt.dds\n[matl_alpha]\n2\n[texture]\nline.dds\n[matl_alpha]\n2\n\
             [profile]\n0\n[profilepnt]\n-4.3\n0.1\n0\n1\n[profilepnt]\n4.3\n0.1\n1\n1\n\
             [profile]\n1\n[profilepnt]\n-0.11\n0.105\n0\n1\n[profilepnt]\n0.11\n0.105\n1\n1\n",
    ));
    for msaa in [1, 4] {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let mut renderer = pollster::block_on(Renderer::new_with(&instance, None,
            Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            omsi_render::RenderOptions { msaa, ssao: false, shadow_size: 1024,
                fxaa: false, render_scale: 1.0, ..Default::default() },
        )).expect("test renderer");
        let mut scene = renderer.new_scene();
        let plane = |half_width: f32| MeshData {
            positions: vec![glam::Vec3::new(-half_width, -1000.0, 0.0), glam::Vec3::new(half_width, -1000.0, 0.0),
                glam::Vec3::new(half_width, 1000.0, 0.0), glam::Vec3::new(-half_width, 1000.0, 0.0)],
            normals: vec![glam::Vec3::Z; 4], uvs: vec![glam::Vec2::ZERO; 4],
            ranges: vec![(0, 6, 0)], indices: vec![0, 1, 2, 0, 2, 3], one_sided: false,
        };
        let mut combined = plane(1000.0);
        let stripe = plane(0.11);
        combined.positions.extend(stripe.positions.iter().map(|p| *p + glam::Vec3::Z * 0.005));
        combined.normals.extend(stripe.normals);
        combined.uvs.extend(stripe.uvs);
        combined.indices.extend(stripe.indices.iter().map(|i| i + 4));
        combined.ranges.push((6, 6, 1));
        let (road_data, line_data) = split_spline_paint(&combined, &def);
        let road_mesh = renderer.add_mesh(&mut scene, &road_data);
        let line_mesh = renderer.add_mesh(&mut scene, &line_data);
        let blue = renderer.add_material_extra(&mut scene, None, AlphaMode::Blend,
            [0.0, 0.0, 1.0, 1.0], true, None, None, None, None, [0.0; 3],
            MaterialExtra { no_z_write: true, ..Default::default() });
        let red = renderer.add_material_extra(&mut scene, None, AlphaMode::Blend,
            [1.0, 0.0, 0.0, 1.0], true, None, None, None, None, [0.0; 3],
            MaterialExtra { no_z_write: true, ..Default::default() });
        let green = renderer.add_material(&mut scene, None, AlphaMode::Opaque, [0.0, 1.0, 0.0, 1.0], true);
        let road = renderer.add_surface_instance(&mut scene, road_mesh, DVec3::ZERO, Mat4::IDENTITY, vec![blue]);
        scene.instances[road].render_phase = RenderPhase::Spline;
        scene.instances[road].blend_sort_origin = Some(DVec3::ZERO);
        let line = renderer.add_surface_instance(&mut scene, line_mesh, DVec3::ZERO, Mat4::IDENTITY, vec![blue, red]);
        scene.instances[line].render_phase = RenderPhase::BeforeNormal;
        scene.instances[line].blend_sort_origin = Some(DVec3::new(0.0, 0.01, 0.0));
        let bridge = renderer.add_surface_instance(&mut scene, road_mesh, DVec3::new(0.0, 0.0, 0.03), Mat4::IDENTITY, vec![green]);
        scene.instances[bridge].render_phase = RenderPhase::Surface;
        let lighting = omsi_render::Lighting { shadows: false, fog_density: 0.0, ..Default::default() };
        for y in [-20.0, 20.0] {
            let camera = omsi_render::Camera { position: DVec3::new(0.0, y, 2.0),
                yaw: if y < 0.0 { 0.0 } else { 180.0 }, pitch: -(2.0f32 / 20.0).atan().to_degrees(),
                roll: 0.0, fov_deg: 60.0, near: 0.1, far: 2000.0 };
            for raised in [false, true] {
                renderer.set_params(&mut scene, bridge, &[], raised, &[]);
                let rgba = renderer.render_to_image(&mut scene, 129, 129, &camera, &lighting).unwrap();
                let pixel = &rgba[(64 * 129 + 64) * 4..][..3];
                let wanted = if raised { 1 } else { 0 };
                assert!(pixel[wanted] > 200 && pixel[2] < 20 && pixel[1 - wanted] < 20,
                    "msaa {msaa}, camera y {y}, raised {raised}: {pixel:?}");
            }
        }
    }
}

#[test]
fn only_flat_blended_ground_strips_draw_after_the_roads() {
    use omsi_scenery::sli::{Spline, SplineProfile, SplineProfilePoint, SplineTexture};
    let line = Spline {
        textures: vec![SplineTexture { alpha: 2, ..Default::default() }],
        profiles: vec![SplineProfile { texture: 0, points: vec![
            SplineProfilePoint { x: -0.11, z: 0.1, ..Default::default() },
            SplineProfilePoint { x: 0.11, z: 0.1, ..Default::default() },
        ] }], ..Default::default()
    };
    assert_eq!(spline_render_phase(&line), RenderPhase::BeforeNormal);
    let mut road = line.clone();
    road.profiles[0].points[0].x = -4.3;
    road.profiles[0].points[1].x = 4.3;
    assert_eq!(spline_render_phase(&road), RenderPhase::Spline);
    let mut solid = line.clone();
    solid.textures[0].alpha = 0;
    assert_eq!(spline_render_phase(&solid), RenderPhase::Spline);
    let mut fence = line.clone();
    fence.profiles[0].points[1].z = 1.0;
    assert_eq!(spline_render_phase(&fence), RenderPhase::Spline);
    let mut wire = line.clone();
    for p in &mut wire.profiles[0].points { p.z = 5.0; }
    assert_eq!(spline_render_phase(&wire), RenderPhase::Spline);
    let mut mixed = line.clone();
    mixed.profiles.push(road.profiles[0].clone());
    assert_eq!(spline_render_phase(&mixed), RenderPhase::Spline);
    let mut traffic = line.clone();
    traffic.paths.push(Default::default());
    assert_eq!(spline_render_phase(&traffic), RenderPhase::Spline);
    let mut rail = line.clone();
    rail.rail_enh.push(Default::default());
    assert_eq!(spline_render_phase(&rail), RenderPhase::Spline);
    let mut unused = line;
    unused.textures.push(SplineTexture::default());
    assert_eq!(spline_render_phase(&unused), RenderPhase::BeforeNormal,
        "an unused opaque texture does not change a painted strip");
    assert_eq!(spline_render_phase(&Spline::default()), RenderPhase::Spline);
}

#[test]
fn mixed_spline_keeps_asphalt_and_painted_ranges_apart() {
    let def = omsi_scenery::sli::Spline::parse(&omsi_cfg::CfgFile::from_str("mixed.sli",
        "[texture]\nasphalt.dds\n[matl_alpha]\n2\n[texture]\nline.dds\n[matl_alpha]\n2\n\
             [profile]\n0\n[profilepnt]\n-2.15\n0.1\n0\n1\n[profilepnt]\n2.15\n0.1\n1\n1\n\
             [profile]\n1\n[profilepnt]\n1.23\n0.105\n0\n1\n[profilepnt]\n1.45\n0.105\n1\n1\n"));
    let src = MeshData {
        positions: vec![glam::Vec3::ZERO, glam::Vec3::X, glam::Vec3::Y,
            glam::Vec3::Z, glam::Vec3::ONE, glam::Vec3::new(1.0, 1.0, 0.0)],
        normals: vec![glam::Vec3::Z; 6], uvs: vec![glam::Vec2::ZERO; 6],
        indices: vec![0, 1, 2, 3, 4, 5], ranges: vec![(0, 3, 0), (3, 3, 1)], one_sided: true,
    };
    let (road, paint) = split_spline_paint(&src, &def);
    assert_eq!(road.ranges.iter().map(|r| r.2).collect::<Vec<_>>(), vec![0]);
    assert_eq!(paint.ranges.iter().map(|r| r.2).collect::<Vec<_>>(), vec![1]);
    assert_eq!(road.indices.len(), 3);
    assert_eq!(paint.indices.len(), 3);
    assert_eq!(paint.positions, src.positions[3..]);
    assert_eq!(paint.normals, src.normals[3..]);
    assert_eq!(paint.uvs, src.uvs[3..]);
    assert!(road.one_sided && paint.one_sided);
}

/// A `[variable_terrainlightmap]` tile's light map is baked as Omsi.exe bakes it: over the
/// tile and its neighbours, north up, colour x min(1, (core / distance)^2) from the lamp's
/// own height, added up, held at 1 and truncated, nothing beyond 15.96 cores.
#[test]
fn a_light_map_is_baked_from_the_lamps_round_the_tile() {
    let ts = tile_size();
    let origin = DVec3::new(10.0 * ts, -4.0 * ts, 0.0);
    let texel = 3.0 * ts / 256.0;
    let at = |img: &omsi_texture::Image, c: usize, r: usize| {
        let i = (r * 256 + c) * 4;
        [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]
    };
    // texel (c, r) lies at x = (3c / 256 - 1) ts, y = (2 - 3r / 256) ts
    let world = |c: usize, r: usize| (origin.x + c as f64 * texel - ts, origin.y + 2.0 * ts - r as f64 * texel);
    let lamp = |c: usize, r: usize, height: f32, color: [f32; 3], radius: f32| {
        let (x, y) = world(c, r);
        BakeLamp { x, y, height, color, radius }
    };
    // a lamp at the ground over a texel of the tile's own third: its full colour there
    let img = bake_light_map(&[lamp(100, 120, 0.0, [0.5, 0.25, 1.0], 3.0)], origin);
    assert_eq!(at(&img, 100, 120), [127, 63, 255]);
    // six metres up with a three metre core: a quarter of it under it (63.75 → 63)
    let img = bake_light_map(&[lamp(100, 120, 6.0, [1.0, 1.0, 1.0], 3.0)], origin);
    assert_eq!(at(&img, 100, 120), [63, 63, 63]);
    // north is the top row: a lamp in the northern neighbour lights a row above the
    // tile's third, and two of them add up to white, not beyond
    let img = bake_light_map(
        &[lamp(128, 40, 0.0, [0.75, 0.0, 0.0], 3.0), lamp(128, 40, 0.0, [0.75, 0.0, 0.0], 3.0)],
        origin,
    );
    assert_eq!(at(&img, 128, 40), [255, 0, 0]);
    assert_eq!(at(&img, 128, 128), [0, 0, 0]);
    // a bright lamp lights up to 15.96 cores along an axis and no further
    let core = 2.0;
    let near = (15.9 * core / texel as f32).floor() as usize;
    let img = bake_light_map(&[lamp(128, 128, 0.0, [100.0, 0.0, 0.0], core)], origin);
    assert!(at(&img, 128 + near, 128)[0] > 0);
    assert!(at(&img, 128, 128 - near)[0] > 0);
    assert_eq!(at(&img, 128 + near + 1, 128)[0], 0);
    assert_eq!(at(&img, 128, 128 - near - 1)[0], 0);
}

#[test]
fn scenery_render_types_map_to_the_cpp_pass_order() {
    use omsi_scenery::sco::RenderType as ScoPhase;
    for (source, expected) in [
        (ScoPhase::PreSurface, RenderPhase::PreSurface),
        (ScoPhase::Surface, RenderPhase::Surface),
        (ScoPhase::OnSurface, RenderPhase::OnSurface),
        (ScoPhase::BeforeNormal, RenderPhase::BeforeNormal),
        (ScoPhase::Normal, RenderPhase::Normal),
        (ScoPhase::AfterNormal, RenderPhase::AfterNormal),
        (ScoPhase::AfterVehicles, RenderPhase::AfterVehicles),
    ] {
        assert_eq!(scenery_render_phase(source), expected);
    }
}

/// A film modelled as a copy of the floor's faces with a slot of its own is an overlay;
/// a panel beside the floor, sharing one edge with it, is not.
#[test]
fn a_copy_of_another_slots_faces_is_an_overlay() {
    let v = glam::Vec3::new;
    let mesh = MeshData {
        positions: vec![v(0.0, 0.0, 0.0), v(4.0, 0.0, 0.0), v(4.0, 2.0, 0.0), v(0.0, 2.0, 0.0), v(4.0, 0.0, 1.0), v(0.0, 0.0, 1.0)],
        // slot 0 the floor, slot 1 the film over it (the same corners), slot 2 a wall
        // standing on the floor's front edge
        indices: vec![0, 1, 2, 0, 2, 3, 0, 1, 2, 0, 2, 3, 0, 1, 4, 0, 4, 5],
        ranges: vec![(0, 6, 0), (6, 6, 1), (12, 6, 2)],
        ..Default::default()
    };
    assert!(slot_overlays_another(&mesh, 1));
    assert!(!slot_overlays_another(&mesh, 2));
}

/// A path's trafficdensity rules per group: the last of each, and traffic on the lane
/// while any group drives there (a path without a rule for the first group has its
/// medium density).
#[test]
fn path_densities_per_group() {
    let rule = |path: i32, value: f64, extra: f64| omsi_map::MapRule {
        path_index: path,
        kind: "trafficdensity".into(),
        value,
        extra,
        ..Default::default()
    };
    let rules = [rule(0, 0.0, 0.0), rule(0, 1.0, 4.0), rule(0, 0.5, 4.0), rule(1, 2.0, 0.0)];
    assert_eq!(path_densities(&rules, 0), (0.5, vec![(0, 0.0), (4, 0.5)]));
    assert_eq!(path_densities(&rules, 1), (2.0, vec![(0, 2.0)]));
    assert_eq!(path_densities(&[rule(2, 0.3, 4.0)], 2), (1.0, vec![(4, 0.3)]));
    assert_eq!(path_densities(&[], 0), (1.0, vec![]));
}

/// A wire strung 5.5 m over its spline is no ground; a wall standing on it, or a
/// catenary spline that has a track bed at the bottom, is.
#[test]
fn only_splines_all_overhead_leave_the_ground() {
    use omsi_scenery::sli::{Spline, SplineProfile, SplineProfilePoint};
    let prof = |zs: &[f32]| SplineProfile { texture: 0, points: zs.iter().map(|&z| SplineProfilePoint { x: z, z, ..Default::default() }).collect() };
    let def = |ps: Vec<SplineProfile>| Spline { profiles: ps, ..Default::default() };
    assert!(overhead_only(&def(vec![prof(&[5.5, 5.6]), prof(&[2.0, 2.0])])));
    assert!(!overhead_only(&def(vec![prof(&[0.0, 2.4])])));
    assert!(!overhead_only(&def(vec![prof(&[5.5, 5.6]), prof(&[-0.2, 0.0])])));
    assert!(!overhead_only(&def(vec![])));
}

/// The car park's first string picks the list; anything that is no number is list 0.
#[test]
fn a_car_park_picks_its_parklist_by_its_first_string() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(parklist_index(&s(&[])), 0);
    assert_eq!(parklist_index(&s(&["2", "x"])), 2);
    assert_eq!(parklist_index(&s(&[" 1 "])), 1);
    assert_eq!(parklist_index(&s(&["Taxi"])), 0);
}

/// A `[terrainmapping]` slot (TH_Wald's Fels01: rock in slot 0, grass top in slot 1)
/// leaves the object's own mesh and comes back in tile space, where the ground under the
/// placed object is: turned a quarter, 10 m into a tile whose corner is at 300/600.
#[test]
fn terrain_mapped_slots_split_off_in_tile_space() {
    let mut src = MeshData::default();
    for p in [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 2.0], [3.0, 0.0, 2.0], [0.0, 3.0, 2.0]] {
        src.positions.push(glam::Vec3::from_array(p));
        src.normals.push(glam::Vec3::Z);
        src.uvs.push(glam::Vec2::ZERO);
    }
    src.indices = vec![0, 1, 2, 3, 4, 5];
    src.ranges = vec![(0, 3, 0), (3, 3, 1)];
    src.one_sided = true;
    let origin = DVec3::new(300.0, 600.0, 0.0);
    let pos = DVec3::new(310.0, 620.0, 5.0);
    let xf = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let (rest, ground) = split_terrain_mapped(&src, &[1], pos, xf, origin);
    assert_eq!(rest.ranges, vec![(0, 3, 0)]);
    assert_eq!(ground.ranges, vec![(0, 3, 0)]);
    assert!(ground.one_sided);
    assert_eq!(ground.positions, src.positions[3..6].to_vec());
    // (3, 0) turned a quarter is (0, 3): 10/23 m into the tile
    let uv = ground.uvs[1] * tile_size() as f32;
    assert!((uv - glam::Vec2::new(10.0, 23.0)).length() < 1e-3, "{uv:?}");
    let (_, none) = split_terrain_mapped(&src, &[2], pos, xf, origin);
    assert!(none.is_empty() && none.ranges.is_empty());
}

/// The Spandau neon lamp (Streetobjects_RUE/neonlight_M_whip_S.sco) declares its glow
/// after the far mesh of `[LOD] 0`: it still glows, near or far.
#[test]
fn lights_of_lower_lods_count() {
    let text = "[LOD]\n0.15\n[mesh]\nnear.o3d\n[LOD]\n0\n[mesh]\nfar.o3d\n[light_enh_2]\n-2.965\n0\n7.170\n-0.3817\n0\n-1.5344\n-1.5344\n0\n0.3817\n0\n1\n230\n230\n255\n2.0\n120\n200\nNightlightA\n0.8\n0.5\n1\n1\n0.2\nlichteffekt1.bmp\n";
    let model = Model::parse(&omsi_cfg::CfgFile::from_str("lamp.sco", text));
    assert_eq!(model.lods.len(), 2);
    assert!(model.lod_meshes(0)[0].light_enh_2.is_empty());
    let asked = Mutex::new(Vec::new());
    let coronas = model_lights_faded(
        &model,
        &|_| Mat4::IDENTITY,
        DVec3::new(100.0, 200.0, 30.0),
        &|v| {
            asked.lock().push(v.to_string());
            1.0
        },
        &[],
    );
    // the glow, its star (effect bit 1), the halo round it in fog and (its cone flag set)
    // the light cone it throws there
    assert_eq!(coronas.len(), 4);
    assert!(!coronas[0].beam && !coronas[0].halo && coronas[0].flags & 8 == 0);
    assert!(coronas[1].flags & 8 != 0 && coronas[1].rotating == 2);
    assert!(coronas[2].halo && coronas[3].beam);
    assert_eq!(asked.into_inner(), vec!["NightlightA".to_string()]);
    assert!((coronas[0].position - DVec3::new(97.035, 200.0, 37.17)).length() < 1e-3);
    assert!(
        coronas[0].direction.z < -0.9,
        "points down: {:?}",
        coronas[0].direction
    );
}

#[test]
fn small_instrument_lights_keep_their_small_size() {
    let text = "[mesh]\ndash.o3d\n[light_enh]\n0\n0\n0\n255\n0\n0\n0.01\nspeedo_warn\n0\n";
    let model = Model::parse(&omsi_cfg::CfgFile::from_str("bus.cfg", text));
    let coronas = model_lights_faded(&model, &|_| Mat4::IDENTITY, DVec3::ZERO, &|_| 1.0, &[]);
    // (effect 0: the glow, and the halo it has in fog)
    let glows: Vec<_> = coronas.iter().filter(|c| !c.halo).collect();
    assert_eq!(glows.len(), 1);
    assert!((glows[0].size - 0.005).abs() < 1e-6);
}

/// The MAN NL's stop request lamp (`model_EN92.cfg`): a `[light_enh]` is drawn as
/// Omsi.exe draws a `[light_enh_2]` - its own bitmap, 5 cm towards the viewer, its
/// brightness factor and its effect bits (3: a star, no halo in fog) - not as a bare
/// glow in its dome (#1159).
#[test]
fn a_light_enh_has_its_bitmap_z_offset_factor_and_effects() {
    let dir = std::env::temp_dir().join("openomsi-light-enh-test");
    std::fs::create_dir_all(dir.join("MAN_NL_NG/model")).unwrap();
    std::fs::create_dir_all(dir.join("MAN_NL_NG/Texture")).unwrap();
    std::fs::write(dir.join("MAN_NL_NG/Texture/D92_Haltewunsch.bmp"), b"BM").unwrap();
    let lamp = |factor: &str, effect: &str| {
        format!("[mesh]\npanel.o3d\n[light_enh]\n-0.635\n5.435\n1.288\n255\n150\n0\n0.05\nhaltewunschlampe_all\n{factor}\n0.05\n{effect}\n0.05\nD92_Haltewunsch.bmp\n")
    };
    let path = dir.join("MAN_NL_NG/model/model_EN92.cfg");
    let model = Model::parse(&omsi_cfg::CfgFile::from_str(&path, &lamp("1", "3")));
    let coronas = model_lights_faded(&model, &|_| Mat4::IDENTITY, DVec3::ZERO, &|_| 1.0, &[]);
    assert_eq!(coronas.len(), 2, "the glow and its star, no fog halo: {coronas:?}");
    let (glow, star) = (&coronas[0], &coronas[1]);
    assert_ne!(glow.texture, crate::lights::glow_texture_id(), "the lamp's own picture");
    assert_eq!(glow.texture, crate::lights::corona_texture_id(&dir.join("MAN_NL_NG/model"), "D92_Haltewunsch.bmp"));
    assert!((glow.z_offset - 0.05).abs() < 1e-6 && glow.rotating == 2);
    assert!((glow.size - 0.025).abs() < 1e-6 && (glow.brightness - 1.0).abs() < 1e-6);
    assert!(star.flags & 8 != 0 && (star.size - 0.0625).abs() < 1e-6);
    // the factor scales the lamp (OMSI: variable times factor), effect 0 has the halo
    let model = Model::parse(&omsi_cfg::CfgFile::from_str(&path, &lamp("0.5", "0")));
    let coronas = model_lights_faded(&model, &|_| Mat4::IDENTITY, DVec3::ZERO, &|_| 1.0, &[]);
    assert_eq!(coronas.len(), 2);
    assert!((coronas[0].brightness - 0.5).abs() < 1e-6 && !coronas[0].halo);
    assert!(coronas[1].halo);
    // switched off: nothing
    assert!(model_lights_faded(&model, &|_| Mat4::IDENTITY, DVec3::ZERO, &|_| 0.0, &[]).is_empty());
}

/// A bridge deck high over the ground under it (London Bridge over the Thames) is
/// draped by its `[crossing_heightdeformation]` as one lying on the ground is: Omsi.exe
/// does it in the object's own frame, wherever the map puts the object (#961).
#[test]
fn a_crossing_far_over_the_ground_is_draped_by_its_height_field() {
    let dir = std::env::temp_dir().join(format!("openomsi-deck-warp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("global.cfg"), "[name]\nDeck\n").unwrap();
    std::fs::write(dir.join("deck.sco"), "[surface]\n[mesh]\ndeck.x\n[crossing_heightdeformation]\ndeck_def.x\n").unwrap();
    // a 10 m square deck at 0, and its field 1 m higher at the far end (y up in a .x)
    let quad = |name: &str, far: f32| {
        format!("xof 0303txt 0032\nMesh {name} {{\n 4;\n 0;0;0;,\n 10;0;0;,\n 10;{far};10;,\n 0;{far};10;;\n 2;\n 3;0,2,1;,\n 3;0,3,2;;\n}}\n")
    };
    std::fs::write(dir.join("deck.x"), quad("deck", 0.0)).unwrap();
    std::fs::write(dir.join("deck_def.x"), quad("field", 1.0)).unwrap();
    let world = World::open(&dir, &dir.join("global.cfg"), 20261001).unwrap();
    let ot = world.object_type("deck.sco").expect("the deck loads");
    assert!(ot.deform.is_some());
    let staged = |z: f64| StagedTile {
        tx: 0,
        ty: 0,
        origin: DVec3::ZERO,
        path: dir.join("tile_0_0.map"),
        base_terrain: Terrain::flat(),
        align: Vec::new(),
        hole_rims: Vec::new(),
        water: None,
        bakes_light_map: false,
        splines: Vec::new(),
        meshes: Mutex::new(Some(Vec::new())),
        drive: Vec::new(),
        lanes: Mutex::new(Vec::new()),
        street_points: Vec::new(),
        sound_lines: Vec::new(),
        objects: vec![StagedObject {
            ot: ot.clone(),
            id: 1,
            place: Placement::Ground { x: 100.0, y: 100.0, z, rot: [0.0; 3] },
            rules: Vec::new(),
            extra: Vec::new(),
            lamp_parent: None,
            parked: false,
            map_object: true,
            instance: 0,
            key: 1,
        }],
        anchors: Vec::new(),
        counts: LoadStats::default(),
        resolved: std::sync::OnceLock::new(),
    };
    // on the ground and 20 m over it, alike
    for z in [0.0, 20.0] {
        let src: HashMap<(i32, i32), Arc<StagedTile>> = [((0, 0), Arc::new(staged(z)))].into_iter().collect();
        let warped = world.warp_crossings(&src[&(0, 0)], &src);
        let deck = warped.get(&0).unwrap_or_else(|| panic!("the deck {z} m over the ground is draped"));
        let top = deck[0].positions.iter().map(|p| p.z).fold(f32::MIN, f32::max);
        assert!((top - 1.0).abs() < 1e-3, "its far end is raised by the field: {top}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_retexture_using_the_originals_model_shows_its_own_textures() {
    let dir = std::env::temp_dir().join(format!("openomsi-retexture-{}", std::process::id()));
    for d in ["orig/model", "orig/texture", "retex/model", "retex/texture"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join("global.cfg"), "[name]\nRetexture\n").unwrap();
    let plate = "xof 0303txt 0032\nMesh plate {\n 3;\n 0;0;0;,\n 1;0;0;,\n 0;1;0;;\n 1;\n 3;0,1,2;;\n MeshMaterialList {\n  1;\n  1;\n  0;;\n  Material { 1;1;1;1;; 0; 0;0;0;; 0;0;0;; TextureFilename { \"plate.bmp\"; } }\n }\n}\n";
    std::fs::write(dir.join("orig/model/plate.x"), plate).unwrap();
    std::fs::write(dir.join("retex/model/plate.x"), plate).unwrap();
    std::fs::write(dir.join("orig/model/plate.cfg"), "[mesh]\nplate.x\n").unwrap();
    std::fs::write(dir.join("orig/texture/plate.bmp"), b"original").unwrap();
    std::fs::write(dir.join("retex/texture/plate.bmp"), b"retexture").unwrap();
    std::fs::write(dir.join("orig/orig.sco"), "[model]\nmodel\\plate.cfg\n").unwrap();
    std::fs::write(dir.join("retex/retex.sco"), "[model]\n..\\orig\\model\\plate.cfg\n").unwrap();
    let world = World::open(&dir, &dir.join("global.cfg"), 20261003).unwrap();
    let found = |sco: &str| {
        let ot = world.object_type(sco).expect("the object loads");
        let dirs = ot.texture_dirs(&dir);
        let dirs: Vec<&Path> = dirs.iter().map(|d| d.as_path()).collect();
        omsi_texture::find_texture(&ot.meshes[0].1[0].texture, &dirs).and_then(|p| std::fs::read(p).ok())
    };
    assert_eq!(found("orig/orig.sco").as_deref(), Some(&b"original"[..]));
    // the copy's own `texture` folder, as Omsi.exe takes it, not the model file's
    assert_eq!(found("retex/retex.sco").as_deref(), Some(&b"retexture"[..]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scripted_lamp_channels_override_stock_phases_and_switch_led_materials() {
    // Numazu's pedestrian script shows green at phase 5, where stock car lamps
    // are red/yellow; it also switches the green mesh via its Yellow blink output.
    let stock_green = standard_traffic_lamp("green", true, true, false, false);
    let green = traffic_lamp_value("green", Some(1.0), stock_green);
    assert!(change_picks_item(green));
    let stock_yellow = standard_traffic_lamp("yellow", false, true, false, false);
    let blink_off = traffic_lamp_value("yellow", Some(0.0), stock_yellow);
    assert!(!change_picks_item(blink_off));
    assert!(change_picks_item(traffic_lamp_value("yellow", Some(1.0), stock_yellow)));
    // A missing script keeps the safe stock fallback and unknown channels stay off.
    assert_eq!(traffic_lamp_value("green", None, stock_green), 0.0);
    assert_eq!(traffic_lamp_value("custom_channel", None, None), 0.0);
    assert_eq!(traffic_lamp_value("1", None, None), 1.0);
}

#[test]
fn standard_traffic_lamps_are_state_driven() {
    assert_eq!(standard_traffic_lamp("red", true, false, false, false), Some(1.0));
    assert_eq!(standard_traffic_lamp("YELLOW", false, true, false, false), Some(1.0));
    assert_eq!(standard_traffic_lamp("green", false, false, true, false), Some(1.0));
    assert_eq!(standard_traffic_lamp("red", false, true, false, false), Some(0.0));
    assert_eq!(standard_traffic_lamp("custom_channel", true, true, true, false), None);
}

#[test]
fn scenery_mouseevent_hit_and_trigger() {
    let dir = std::env::temp_dir().join(format!("openomsi-scenery-mouseevent-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("global.cfg"), "[map]\n0\n0\n0\n").unwrap();
    let world = World::open(&dir, &dir.join("global.cfg"), 20261001).unwrap();

    let mut mesh_data = omsi_geometry::MeshData::default();
    mesh_data.positions = vec![
        glam::Vec3::new(-1.0, 5.0, -1.0),
        glam::Vec3::new(1.0, 5.0, -1.0),
        glam::Vec3::new(1.0, 5.0, 1.0),
        glam::Vec3::new(-1.0, 5.0, 1.0),
    ];
    mesh_data.indices = vec![0, 1, 2, 0, 2, 3];

    let mut model = omsi_model::Model::default();
    let mut mdef = omsi_model::MeshDef::default();
    mdef.mouse_event = Some("toggle_switch".to_string());
    model.meshes.push(mdef);

    let mut prog = omsi_script::Program::default();
    let var_id = prog.declare_var("Switch");
    prog.blocks.push(omsi_script::compile::Block {
        name: "toggle_switch".into(),
        ops: vec![
            omsi_script::Op::Load(var_id),
            omsi_script::Op::Not,
            omsi_script::Op::Store(var_id),
        ],
        ..Default::default()
    });
    prog.triggers.insert("toggle_switch".into(), 0);
    prog.blocks.push(omsi_script::compile::Block {
        name: "toggle_switch_drag".into(),
        ops: vec![
            omsi_script::Op::LoadSys(omsi_script::SysVar::MouseX),
            omsi_script::Op::Store(var_id),
        ],
        ..Default::default()
    });
    prog.triggers.insert("toggle_switch_drag".into(), 1);
    prog.blocks.push(omsi_script::compile::Block {
        name: "toggle_switch_off".into(),
        ops: vec![
            omsi_script::Op::Push(0.0),
            omsi_script::Op::Store(var_id),
        ],
        ..Default::default()
    });
    prog.triggers.insert("toggle_switch_off".into(), 2);

    let inst = omsi_sim::scenery::SceneryInstance::new(
        Arc::new(prog),
        &[],
        omsi_sim::SimClock::default(),
        &[],
    );

    let ot = Arc::new(ObjectType {
        sco: omsi_scenery::sco::SceneryObject::default(),
        sound_path: Default::default(),
        model,
        model_dir: dir.clone(),
        meshes: vec![(mesh_data, Vec::new(), Vec::new())],
        mesh_visible: vec![None],
        mesh_def_index: vec![0],
        mesh_pivots: vec![glam::Mat4::IDENTITY],
        mesh_shadow: vec![false],
        mesh_casts: vec![false],
        has_mouse_events: true,
        editor_pick_bounds: Default::default(),
        embedded_lights: Default::default(),
        program: None,
        lower_lods: Vec::new(),
        lod0_min: 0.0,
        paint_scheme_count: 0,
        dynamic_textures: Vec::new(),
        holes: Vec::new(),
        deform: None,
        collision: None,
        paint: false,
        camera: Default::default(),
        collision_shape: Default::default(),
    });

    world.scripted.lock().push(ScriptedObject {
        ty: ot,
        pos: DVec3::ZERO,
        xf: glam::Mat4::IDENTITY,
        instances: vec![0],
        inst,
        controller: None,
        light_index: 0,
        light_parent: None,
        map_id: 42,
        variants: Vec::new(),
        sounds: None,
        tile: (0, 0),
        var_parent: None,
        texts: Vec::new(),
        arrivals: false,
        htmls: Vec::new(),
        alpha_slots: Vec::new(),
        alpha_last: Vec::new(),
    });

    // 1. Raycast towards (0, 1, 0) should hit the quad at (0, 5, 0)
    let hit = world.scenery_object_hit(DVec3::ZERO, glam::Vec3::Y, 50.0, 0.0);
    assert!(hit.is_some(), "scenery object hit should find the switch");
    let h = hit.unwrap();
    assert_eq!(h.map_id, 42);
    assert_eq!(h.event, "toggle_switch");
    assert!((h.t - 5.0).abs() < 1e-3);

    // 2. Raycast in opposite direction should miss
    let miss = world.scenery_object_hit(DVec3::ZERO, -glam::Vec3::Y, 50.0, 0.0);
    assert!(miss.is_none());

    // 3. Test click triggers toggle_switch: Switch was 0, becomes 1
    assert_eq!(world.scripted.lock()[0].inst.var("Switch"), Some(0.0));
    assert!(world.scenery_object_click(42, "toggle_switch"));
    assert_eq!(world.scripted.lock()[0].inst.var("Switch"), Some(1.0));

    // 4. Test drag triggers toggle_switch_drag with mouse_x
    assert!(world.scenery_object_drag(42, "toggle_switch", 0.75, 0.0));
    assert_eq!(world.scripted.lock()[0].inst.var("Switch"), Some(0.75));

    // 5. Test release triggers toggle_switch_off
    assert!(world.scenery_object_release(42, "toggle_switch"));
    assert_eq!(world.scripted.lock()[0].inst.var("Switch"), Some(0.0));

    let _ = std::fs::remove_dir_all(&dir);
}


#[test]
fn generated_ai_paths_filter_vehicles_and_join_reversed_reference_segments() {
    let settings=crate::traffic_editor::Settings {forward_bus:true,..Default::default()};
    let a=omsi_map::MapSpline {id:1,length:20.0,..Default::default()};
    // Second authored spline faces backwards, but the route continues north.
    let b=omsi_map::MapSpline {id:2,length:20.0,pos:[0.0,40.0,0.0],heading:180.0,..Default::default()};
    let plan=crate::traffic_editor::build(&[(((0,0),1),a,false),(((0,0),2),b,true)],&settings).unwrap();
    let mut network=omsi_sim::traffic::Network::default();
    for ((tile,piece),(_,body)) in plan.pieces.iter().zip(&plan.definitions) {
        let def=Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,body));
        let curve=SplineCurve::from_map(piece,glam::DVec2::ZERO);
        let lanes=lanes::spline_lanes(&def,piece,&curve,*tile);
        assert_eq!(lanes.len(),2);
        for vehicle in [0,1,3] {assert!(!lanes[0].allows(vehicle));assert!(lanes[1].allows(vehicle));}
        for vehicle in [2,-1] {assert!(lanes[0].allows(vehicle));assert!(lanes[1].allows(vehicle));}
        network.lanes.extend(lanes);
    }
    assert!(network.lanes[0].end().distance(network.lanes[2].start())<1e-5);
    assert!(network.lanes[3].end().distance(network.lanes[1].start())<1e-5);
    network.link(0.5);
    assert!(network.lanes[0].next.contains(&2));assert!(network.lanes[3].next.contains(&1));
}

#[test]
fn generated_ai_profile_correction_matches_surface_and_saved_lanes() {
    let correction=omsi_map::ProfileTransition {station:11.398266499412,span:5.699133249706,x:[-4.0,4.0],
        offsets:[[0.500418508815,-0.054123807654,-0.100000001490],[-0.500418508815,0.054123807654,-0.100000001490]]};
    let source=omsi_map::MapSpline {id:9859278,length:correction.station,radius:23.566275784890,
        heading:38.173887354201,profile_transitions:[None,Some(correction)],..Default::default()};
    let settings=crate::traffic_editor::Settings::default();
    let plan=crate::traffic_editor::build(&[(((0,0),source.id),source.clone(),false)],&settings).unwrap();
    let piece=&plan.pieces[0].1;let def=Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,&plan.definitions[0].1));
    let c=SplineCurve::from_map(piece,glam::DVec2::ZERO);
    let lanes=lanes::spline_lanes(&def,piece,&c,(0,0));
    assert_eq!(lanes.len(),2);assert_eq!(piece.profile_transitions,source.profile_transitions);
    let reference=SplineCurve::from_map(&source,glam::DVec2::ZERO);
    for (i,lane) in lanes.iter().enumerate() {
        let x=if i==0 {1.5}else{-1.5};
        let endpoint=reference.offset_point(source.length,x,0.1);
        assert!(endpoint.distance(if i==0{lane.end()}else{lane.start()})<1e-6);
        assert!(plan.endpoints.iter().any(|(p,_,_)|p.distance(endpoint)<1e-6));
    }
}

#[test]
fn generated_ai_cant_skew_matches_surface_preview_and_runtime() {
    let source_def=Spline::parse(&omsi_cfg::CfgFile::from_str("test-cant-skew-v14.sli","[halfcantwidth]\n1.25\n"));
    omsi_geometry::register_half_cant_width("test-cant-skew-v14.sli",&source_def);
    let source=omsi_map::MapSpline {id:722334,file:"test-cant-skew-v14.sli".into(),length:24.0,radius:80.0,
        cant_start:-6.0,cant_end:12.0,skew_start:0.4,skew_end:-0.3,grad_start:2.0,grad_end:4.0,
        profile_transitions:[None,Some(omsi_map::ProfileTransition {station:24.0,span:4.0,x:[-4.0,4.0],
            offsets:[[0.2,0.05,-0.1],[-0.2,-0.05,-0.1]]})],..Default::default()};
    for backwards in [false,true] {
        let settings=crate::traffic_editor::Settings::default();
        let plan=crate::traffic_editor::build(&[(((0,0),source.id),source.clone(),backwards)],&settings).unwrap();
        let piece=&plan.pieces[0].1;
        let def=Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,&plan.definitions[0].1));
        assert_eq!(def.half_cant_width,source_def.half_cant_width);
        let c=SplineCurve::from_map(piece,glam::DVec2::ZERO);
        let reference=SplineCurve::from_map(&source,glam::DVec2::ZERO);
        for (lane,(x,dir,_)) in lanes::spline_lanes(&def,piece,&c,(0,0)).iter().zip(crate::traffic_editor::lanes(&settings,backwards)) {
            for (j,point) in lane.points.iter().enumerate() {
                let fraction=j as f64/(lane.points.len()-1) as f64;
                let station=source.length*if dir==1 {1.0-fraction}else{fraction};
                let expected=omsi_geometry::spline_profile_point(&source_def,&reference,false,station,x,settings.height);
                assert!(point.distance(expected)<1e-6,"runtime {point:?}, mesh {expected:?}");
            }
            for station in [0.0,12.0,24.0] {
                let expected=omsi_geometry::spline_profile_point(&source_def,&reference,false,station,x,settings.height);
                assert!(plan.markers.iter().any(|(p,_)|p.distance(expected)<1e-6));
            }
        }
    }
}

#[test]
fn generated_ai_canted_skewed_chain_links_both_directions() {
    let a=omsi_map::MapSpline {id:1,length:20.0,cant_start:8.0,cant_end:8.0,skew_start:0.3,skew_end:0.3,..Default::default()};
    let b=omsi_map::MapSpline {id:2,pos:[0.0,20.0,0.0],..a.clone()};
    let plan=crate::traffic_editor::build(&[(((0,0),1),a,false),(((0,0),2),b,false)],&Default::default()).unwrap();
    let mut network=omsi_sim::traffic::Network::default();
    for ((tile,piece),(_,body)) in plan.pieces.iter().zip(&plan.definitions) {
        let def=Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,body));
        network.lanes.extend(lanes::spline_lanes(&def,piece,&SplineCurve::from_map(piece,glam::DVec2::ZERO),*tile));
    }
    network.link(0.5);
    assert!(network.lanes[0].next.contains(&2));assert!(network.lanes[3].next.contains(&1));
}

#[test]
fn generated_ai_freeform_mouse_geometry_and_explicit_connection_survive_definition() {
    use crate::traffic_editor::{Window,Target,build};
    let source=omsi_map::MapSpline {id:1,length:20.0,..Default::default()};
    let roads=vec![(((0,0),1),source.clone(),false)];
    let mut w=Window::new(((0,0),1));w.enable_free();
    w.preview=build(&roads,&w.settings).unwrap();
    w.move_node(4,DVec3::new(6.0,10.0,0.1));
    w.preview=build(&roads,&w.settings).unwrap();
    let end=Target {point:DVec3::new(10.0,25.0,0.1),direction:DVec3::X,start:true,
        key:omsi_sim::traffic::LaneKey {tile:(0,0),id:77,path:0},reversed:false};
    assert!(w.snap_node(8,end));
    w.settings.connections.push((0,false,[0,0,88,0,0]));
    let plan=build(&roads,&w.settings).unwrap();
    let piece=&plan.pieces[0].1;let def=Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,&plan.definitions[0].1));
    assert_eq!(def.editor_path_nodes.len(),10);assert_eq!(def.editor_path_connections.len(),2);
    let generated=lanes::spline_lanes(&def,piece,&SplineCurve::from_map(piece,glam::DVec2::ZERO),(0,0));
    assert!(generated[0].points.iter().any(|p|p.distance(DVec3::new(6.0,10.0,0.1))<1e-6));
    assert!(generated[0].end().distance(end.point)<1e-6);
    for p in &generated[1].points {assert!((p.x+1.5).abs()<1e-6);assert!((p.z-0.1).abs()<1e-6);}
    let mut wanted=omsi_sim::traffic::LaneBuilder::arc(end.point,90.0,20.0,0.0,0.0,LaneKind::Street,3.0);wanted.key=Some(end.key);
    let mut unwanted=wanted.clone();unwanted.key.as_mut().unwrap().id=78;
    let mut net=omsi_sim::traffic::Network::default();net.lanes=vec![generated[0].clone(),wanted.clone(),unwanted.clone()];
    net.link(1.5);assert_eq!(net.lanes[0].next,vec![1]);
    let mut streamed=omsi_sim::traffic::Network::default();streamed.lanes=vec![generated[0].clone()];streamed.link(1.5);
    streamed.extend(vec![wanted,unwanted],1.5);assert_eq!(streamed.lanes[0].next,vec![1]);
    let mut incoming=omsi_sim::traffic::LaneBuilder::arc(generated[0].start()-DVec3::Y*10.0,0.0,10.0,0.0,0.0,LaneKind::Street,3.0);
    incoming.key=Some(omsi_sim::traffic::LaneKey {tile:(0,0),id:88,path:0});
    let mut wrong_incoming=incoming.clone();wrong_incoming.key.as_mut().unwrap().id=89;
    let range=streamed.extend(vec![incoming,wrong_incoming],1.5);
    assert!(streamed.lanes[range.start].next.contains(&0));assert!(!streamed.lanes[range.start+1].next.contains(&0));
    // Moving one endpoint releases only that end's explicit target restriction.
    w.preview=plan;w.move_node(8,DVec3::new(11.0,25.0,0.1));
    assert_eq!(w.settings.connections.len(),1);assert!(!w.settings.connections[0].1);
}

#[test]
fn generated_ai_parallel_lanes_have_independent_directions_and_shapes() {
    let source=omsi_map::MapSpline {length:20.0,..Default::default()};
    let settings=crate::traffic_editor::Settings {parallel:true,nodes:vec![(0,[0.0,0.0,0.0,0.0]),(0,[0.5,2.0,0.0,0.0]),(0,[1.0,0.0,0.0,0.0])],..Default::default()};
    let plan=crate::traffic_editor::build(&[(((0,0),1),source.clone(),false)],&settings).unwrap();
    let p=&plan.pieces[0].1;let def=Spline::parse(&omsi_cfg::CfgFile::from_str(&p.file,&plan.definitions[0].1));
    assert_eq!(def.paths[0].direction,0);assert_eq!(def.paths[1].direction,0);
    let ls=lanes::spline_lanes(&def,p,&SplineCurve::from_map(p,glam::DVec2::ZERO),(0,0));
    assert!(!ls[0].reversed&&!ls[1].reversed);assert!(ls[0].points.iter().any(|p|p.x>3.0));
    assert!(ls[1].points.iter().all(|p|(p.x+1.5).abs()<1e-6));
}
