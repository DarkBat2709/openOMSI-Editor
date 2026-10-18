use super::*;

#[test]
fn editor_keeps_attachment_advertisement_without_editable_object_record() {
    let dir = std::env::temp_dir().join(format!("editor-advert-{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("global.cfg"),"[name]\nAdvert regression\n").unwrap();
    std::fs::write(dir.join("advert.sco"),"[friendlyname]\nAttached advert\n").unwrap();
    let world = World::open(&dir,&dir.join("global.cfg"),20261012).unwrap();
    let ty = world.object_type("advert.sco").unwrap();
    let mut program = omsi_script::Program::default();
    let advert = program.declare_var("Werbung") as usize;
    let program = Arc::new(program);
    let labels = vec!["Hoheneck".to_string()];
    let make = || omsi_sim::scenery::SceneryInstance::new(program.clone(), &[], omsi_sim::SimClock::default(), &labels);
    let mut inst = make(); inst.state.vars[advert] = 5.0;
    world.scripted.lock().push(ScriptedObject {
        map_labels: labels.clone(), ty: ty.clone(), pos: DVec3::ZERO, xf: Mat4::IDENTITY,
        instances: vec![0], inst, controller: None, light_index: 0, light_parent: None,
        map_id: 726561, variants: Vec::new(), sounds: None, tile: (0,0), var_parent: None,
        texts: Vec::new(), arrivals: false, htmls: Vec::new(), alpha_slots: Vec::new(), alpha_last: Vec::new(),
    });
    world.tile_state.lock().entry((0,0)).or_default().gpu.instances.push(0);
    assert!(world.edit_objects.lock().is_empty(),"spline attachments have no editable-object entry");
    let mut scenery = omsi_sim::scenery::SceneryState::default();
    world.scenery_sync(&mut scenery);
    world.capture_editor_scripts(&scenery, &[(0,0)]);
    world.hold_editor_gpu((0,0)).unwrap();
    let saved = world.editor_script_states.lock();
    let saved = saved.get(&((0,0),726561)).expect("attached advertisement must be captured");
    let mut rebuilt = make();
    assert!(saved.restore(&ty.sco.path,&labels,&mut rebuilt));
    assert_eq!(rebuilt.var("Werbung"),Some(5.0));
    assert!(!saved.restore(&ty.sco.path,&["changed label".into()],&mut rebuilt));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn every_object_a_scenery_step_shows_finds_its_view_behind_a_silenced_one() {
    let ot = Arc::new(ObjectType {
        sco: omsi_scenery::sco::SceneryObject::default(),
        sound_path: Default::default(),
        model: omsi_model::Model::default(),
        model_dir: Default::default(),
        meshes: Vec::new(),
        mesh_visible: Vec::new(),
        mesh_def_index: Vec::new(),
        mesh_pivots: Vec::new(),
        mesh_shadow: Vec::new(),
        mesh_casts: Vec::new(),
        has_mouse_events: false,
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
    let mut set = ScriptedSet::default();
    for (map_id, x) in [(1, 0.0), (2, 10.0), (3, omsi_sim::scenery::SCRIPT_RANGE + 20.0)] {
        set.push(ScriptedObject {
            map_labels: Vec::new(),
            ty: ot.clone(),
            pos: DVec3::new(x, 0.0, 0.0),
            xf: glam::Mat4::IDENTITY,
            instances: Vec::new(),
            inst: omsi_sim::scenery::SceneryInstance::new(
                Arc::new(omsi_script::Program::default()),
                &[],
                omsi_sim::SimClock::default(),
                &[],
            ),
            controller: None,
            light_index: 0,
            light_parent: None,
            map_id,
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
    }
    let mut scenery = omsi_sim::scenery::SceneryState::default();
    scenery.take_inbox(&set.inbox);
    let phase = |_: usize, _: usize| (0.0, 0.0);
    let inputs = omsi_sim::scenery::OwnedSceneryInputs::default();
    scenery.step(1.0 / 30.0, &inputs.as_inputs(&phase), &mut Default::default());
    let map_id = |key: u64| view_of(&set.views, key).map(|i| set.views[i].map_id);
    let found: Vec<(bool, Option<i64>)> = scenery
        .outputs()
        .cmds
        .iter()
        .map(|c| match c {
            omsi_sim::scenery::SceneryCmd::Silence(k) => (false, map_id(*k)),
            omsi_sim::scenery::SceneryCmd::Show(s) => (true, map_id(s.key)),
        })
        .collect();
    assert_eq!(found, vec![(false, Some(3)), (true, Some(1)), (true, Some(2))]);
}
