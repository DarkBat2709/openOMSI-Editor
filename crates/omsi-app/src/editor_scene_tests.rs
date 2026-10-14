use omsi_scene::{World,ObjectEdit};
use omsi_map::Terrain;
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
            let data=world.data();let surface=data.ground.surfaces.get(&(0,0)).unwrap();
            assert_eq!(surface.cut_at(80.0,100.0,0.0,0.03),!deleted);
            assert!(surface.cut_at(200.0,100.0,0.0,0.03));
            assert_eq!(std::fs::read(&terrain_path).unwrap(),heights);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }


use omsi_geometry::SplineCurve;
use omsi_scenery::Spline;
use omsi_sim::traffic::LaneKind;
use glam::DVec3;
use omsi_scene::lanes;
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
