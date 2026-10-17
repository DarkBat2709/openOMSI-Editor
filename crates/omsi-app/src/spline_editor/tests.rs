use super::*;

fn parsed(text: &str) -> Tile {
    Tile::parse(&omsi_cfg::CfgFile::from_str("test.map", text))
}

#[test]
fn group_selection_toggles_plain_click_selects_one_and_empty_click_clears() {
    let a=((0,0),1); let b=((1,0),2); let mut editor=SplineEditor::default();
    editor.update_group(vec![a],Some(b),true);
    assert_eq!(editor.selection_keys(),vec![a,b]);
    editor.update_group(editor.selection_keys(),Some(a),false);
    assert_eq!(editor.selection_keys(),vec![a]);
    editor.update_group(editor.selection_keys(),Some(b),true);
    assert_eq!(editor.selection_keys(),vec![a,b]);
    editor.update_group(editor.selection_keys(),Some(a),true);
    assert_eq!(editor.selection_keys(),vec![b]);
    editor.update_group(editor.selection_keys(),None,true);
    assert_eq!(editor.selection_keys(),vec![b]);
    editor.update_group(editor.selection_keys(),None,false);
    assert!(editor.selection_keys().is_empty());
}

#[test]
fn group_shapes_concentric_markings_with_equal_progress_and_height_offsets() {
    let road=MapSpline{id:1,file:"road.sli".into(),pos:[100.0,100.0,5.0],length:20.0,grad_start:2.0,grad_end:4.0,delta_h:Some(0.6),is_h:true,..Default::default()};
    let mut original=vec![(((0,0),1),road.clone())];
    for (id,x) in [(2,-3.0),(3,3.0)] {
        original.push((((0,0),id),MapSpline{id,file:"marking.sli".into(),pos:[100.0+x,100.0,5.01],mirror:true,tex_offset:7.0,..road.clone()}));
    }
    let mut parts=original.clone();
    for action in [Action::Curvature(0.02),Action::Length(5.0),Action::Grade(0.5),Action::Curvature(-0.04),Action::Straight] {
        parts=shaped_group(&parts,&action).unwrap();
        let reference=curve((0,0),&parts[0].1);
        for (index,x) in [(1,-3.0),(2,3.0)] {
            let s=&parts[index].1; let c=curve((0,0),s);
            for step in 0..=20 {
                let t=step as f64/20.0;
                let base=reference.point_at(reference.length*t);
                let d=SplineCurve::dir(reference.heading_at(reference.length*t));
                let expected=base+DVec3::new(d.y*x,-d.x*x,0.01);
                assert!(c.point_at(c.length*t).distance(expected)<1e-7,"{step} / {index}");
            }
            assert!(s.mirror);assert_eq!(s.tex_offset,7.0);
        }
    }
    assert!(parts.iter().all(|(_,s)|s.radius==0.0));
    let rotated=shaped_group(&original,&Action::Turn(30.0)).unwrap();
    assert!((curve((0,0),&rotated[1].1).start.distance(curve((0,0),&rotated[2].1).start)-6.0).abs()<1e-8);
    let bent=shaped_group(&original,&Action::Curvature(0.02)).unwrap();
    let straight=shaped_group(&bent,&Action::Straight).unwrap();
    for (a,b) in original.iter().zip(straight.iter()) {
        assert!(curve(a.0.0,&a.1).end_point().distance(curve(b.0.0,&b.1).end_point())<1e-8);
    }
}

#[test]
fn merge_map_shifted_asphalt_and_marking_layers_can_be_shaped() {
    // Actual map records 715278 / 706831. Their origins differ by 0.169 m,
    // with identical curves rather than mathematically concentric radii.
    let asphalt=MapSpline{id:715278,pos:[169.284678055247,257.891582317737,16.000000436103],
        heading:24.637189976280,length:48.999999058405,radius:59.999999058405,..Default::default()};
    let marking=MapSpline{id:706831,pos:[169.178975866957,258.023324209056,16.000013787543],..asphalt.clone()};
    let a=curve((0,0),&asphalt); let b=curve((0,0),&marking);
    let mut parts=vec![(((0,0),1),between("asphalt.sli",1,(0,0),a.end_point(),a.end_point()+SplineCurve::dir(a.heading_at(a.length)).extend(0.0)*30.0,534.0)),
        (((0,0),2),between("marking.sli",2,(0,0),b.end_point(),b.end_point()+SplineCurve::dir(b.heading_at(b.length)).extend(0.0)*30.0,534.0))];
    // Place the fixture away from the tile border; keep its exact relative origins.
    for (_,s) in &mut parts {s.pos[0]-=100.0;s.pos[1]-=100.0;}
    let offset=curve((0,0),&parts[1].1).start-curve((0,0),&parts[0].1).start;
    for action in [Action::Length(1.0),Action::Curvature(0.01),Action::Grade(0.5),Action::Curvature(-0.02),Action::Straight] {
        parts=shaped_group(&parts,&action).unwrap();
        let a=curve((0,0),&parts[0].1);let b=curve((0,0),&parts[1].1);
        for n in 0..=20 {let t=n as f64/20.0;
            assert!((b.point_at(b.length*t)-a.point_at(a.length*t)-offset).length()<1e-7);
        }
    }
}

#[test]
fn merge_pair_deleted_successors_continue_then_shape_across_existing_tiles() {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let base=std::env::temp_dir().join(format!("omsi-merge-build-{stamp}"));let root=base.join("original");let map=root.join("maps/Test");
    std::fs::create_dir_all(&map).unwrap();
    let asphalt=MapSpline{id:715278,next_id:9860259,file:"asphalt.sli".into(),pos:[169.284678055247,257.891582317737,16.000000436103],
        heading:24.637189976280,length:48.999999058405,radius:59.999999058405,..Default::default()};
    let marking=MapSpline{id:706831,next_id:9860260,file:"marking.sli".into(),pos:[169.178975866957,258.023324209056,16.000013787543],..asphalt.clone()};
    let mut global=String::new();
    for x in 0..=1 {for y in 0..=1 {
        global+=&format!("[map]\n{x}\n{y}\ntile_{x}_{y}.map\n");
        let records=if x==0 && y==0 {format!("{}{}{}{}",record(&asphalt,"0","\n"),record(&marking,"0","\n"),
            record(&MapSpline{id:9860259,file:"asphalt.sli".into(),deleted:true,length:4.0,..Default::default()},"0","\n"),
            record(&MapSpline{id:9860260,file:"marking.sli".into(),deleted:true,length:20.0,..Default::default()},"0","\n"))}else{String::new()};
        std::fs::write(map.join(format!("tile_{x}_{y}.map")),format!("[version]\n14\n{records}")).unwrap();
    }}
    std::fs::write(map.join("global.cfg"),global).unwrap();
    let world=World::open(&root,&map.join("global.cfg"),20000101).unwrap();
    let a=((0,0),asphalt.id);let b=((0,0),marking.id);
    world.spline_edits.lock().originals.extend([(a,asphalt.clone()),(b,marking.clone())]);
    assert!(!deleted_spline_link(&world.spline_edits.lock(),9860260));
    assert!(deleted_spline_link_in_world(&world,(0,0),9860260));
    let mut editor=SplineEditor{selected:Some(b),group:vec![a,b],..Default::default()};
    let msg=editor.apply(&world,&Action::Copy).unwrap();assert!(msg.contains("gemeinsam fortgesetzt"),"{msg}");
    let new_keys=editor.selection_keys();assert_eq!(new_keys.len(),2);
    let baseline=new_keys.iter().map(|k|(*k,world.spline_edits.lock().current(*k).unwrap())).collect::<Vec<_>>();
    for action in [Action::Length(20.0),Action::Curvature(-0.01),Action::Grade(0.5)] {
        let count=editor.undo.len();let msg=editor.apply(&world,&action).unwrap();assert_eq!(editor.undo.len(),count+1,"{msg}");
    }
    assert!(world.spline_edits.lock().dirty_tiles.contains(&(0,1)));
    let content=base.join("content");
    crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&root).unwrap();
    let saved=Tile::load(&content.join("maps/Test/tile_0_0.map")).unwrap();
    for key in &new_keys {let current=world.spline_edits.lock().current(*key).unwrap();
        let s=saved.splines.iter().find(|s|s.id==key.1).unwrap();assert!((s.length-current.length).abs()<1e-8);assert!((s.radius-current.radius).abs()<1e-8);
    }
    for _ in 0..3 {editor.undo(&world);}
    for (key,s) in baseline {assert_eq!(world.spline_edits.lock().current(key).unwrap(),s);}
    editor.undo(&world);
    assert_eq!(world.spline_edits.lock().current(a).unwrap(),asphalt);
    assert_eq!(world.spline_edits.lock().current(b).unwrap(),marking);
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
#[ignore = "Reads the prepared local repair without modifying the user map"]
fn validate_prepared_marking_repair() {
    let root=std::path::Path::new("/home/chris/Games/openOMSI-0.1.1541-linux-x64");
    let world=World::open(root,&root.join("maps/Lemmental_Krefrath_Merge_Test/global.cfg"),20261017).unwrap();
    let dir=std::path::Path::new("/home/chris/Dokumente/ChatGPT/Private Space/Kreisverkehr-Reparatur-20261017");
    let old=Tile::load(&dir.join("tile_-16_-10.map.vorher")).unwrap();
    let new=Tile::load(&dir.join("tile_-16_-10.map.vorschlag")).unwrap();assert_eq!(old.splines.len(),new.splines.len());
    for (r,m) in [(9860281,9860290),(9860282,9860291)] {
        let road=new.splines.iter().find(|s|s.id==r).unwrap();let paint=new.splines.iter().find(|s|s.id==m).unwrap();
        assert!(!paint.deleted);assert_eq!(road.length,paint.length);assert_eq!(road.radius,paint.radius);
        let rd=world.spline_type(&road.file).unwrap();let md=world.spline_type(&paint.file).unwrap();
        for i in 0..=40 {for x in [-3.69,0.0,3.69] {
            let station=road.length*i as f64/40.0;
            let rp=omsi_geometry::spline_profile_point(&rd.def,&curve((-16,-10),road),false,station,x,0.1);
            let mp=omsi_geometry::spline_profile_point(&md.def,&curve((-16,-10),paint),false,station,x,0.105);
            // At the preserved upstream offset the road's spatial height differs slightly;
            // the remaining section shares the asphalt's exact deformation.
            if m==9860291 || i>=20 {assert!(mp.z>rp.z,"{m} station {station}");}
        }}
    }
}

#[test]
fn junction_connects_paint_as_layer_without_stretching_it_to_arm_width() {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root=std::env::temp_dir().join(format!("omsi-junction-layers-{stamp}"));let map=root.join("maps/Test");
    std::fs::create_dir_all(&map).unwrap();std::fs::create_dir_all(root.join("Splines")).unwrap();
    let profile=|a:f64,b:f64,z:f64|format!("[profile]\n0\n[profilepnt]\n{a}\n{z}\n0\n1\n[profilepnt]\n{b}\n{z}\n1\n1\n");
    std::fs::write(root.join("Splines/road.sli"),profile(-5.0,5.0,0.1)).unwrap();
    std::fs::write(root.join("Splines/paint.sli"),profile(-4.0,-3.88,0.105)+&profile(-0.06,0.06,0.105)+&profile(3.88,4.0,0.105)).unwrap();
    std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n").unwrap();
    std::fs::write(map.join("tile_0_0.map"),"[version]\n14\n").unwrap();
    let world=World::open(&root,&map.join("global.cfg"),20000101).unwrap();
    let a=((0,0),1);let b=((0,0),2);
    let road=MapSpline{id:1,file:"Splines/road.sli".into(),pos:[100.0,100.0,5.0],length:30.0,prev_id:10,..Default::default()};
    let paint=MapSpline{id:2,file:"Splines/paint.sli".into(),pos:[100.1,100.1,5.0],prev_id:11,..road.clone()};
    world.spline_edits.lock().originals.extend([(a,road.clone()),(b,paint.clone())]);
    let def=world.spline_type(&road.file).unwrap().def.clone();
    let port=crate::junction_connections::Pose{at:DVec3::new(100.0,134.0,5.0),heading:0.0,tilt:[2.0,1.0]}.port(DVec3::ZERO,180.0).unwrap();
    let editor=SplineEditor::default();
    let mismatched=MapSpline{length:35.0,pos:[100.1,95.1,5.0],..paint.clone()};
    world.spline_edits.lock().changed.insert(b,mismatched);
    assert!(editor.junction_layer_plan(&world,a,99,port,&def).unwrap_err().contains("unterschiedliche Abschnitte"));
    world.spline_edits.lock().changed.remove(&b);
    let plan=editor.junction_layer_plan(&world,a,99,port,&def).unwrap();assert_eq!(plan.len(),2);
    let r=&plan.iter().find(|(k,_)|*k==a).unwrap().1;let m=&plan.iter().find(|(k,_)|*k==b).unwrap().1;
    assert_eq!(r.length,m.length);assert_eq!(r.radius,m.radius);assert_eq!(m.next_id,99);assert_eq!(m.prev_id,11);
    let md=world.spline_type(&paint.file).unwrap();
    for (old,new) in edge_points((0,0),&paint,&md.def,End::Start).iter().zip(edge_points((0,0),m,&md.def,End::Start)) {assert!(old.distance(new)<1e-5);}
    for i in 6..=20 {for x in [-3.94,0.0,3.94] {
        let station=r.length*i as f64/20.0;
        let asphalt=omsi_geometry::spline_profile_point(&def,&curve((0,0),r),false,station,x,0.1);
        let marking=omsi_geometry::spline_profile_point(&md.def,&curve((0,0),m),false,station,x,0.105);
        assert!(marking.z>asphalt.z-0.001,"paint below road at {station}");
    }}
    // Repeating the builder update still treats the two layers as one arm.
    world.spline_edits.lock().changed.extend(plan.clone());
    assert_eq!(editor.junction_layer_plan(&world,b,99,port,&def).unwrap().len(),2);
    // Explicit fitting repairs mismatched length, height and slope without
    // altering the reference, paint identity, texture phase or connections.
    let mut editor=editor;
    let mut bad=paint.clone();bad.pos[2]+=0.7;bad.length=12.0;bad.radius=35.0;
    let mut reference=road.clone();reference.radius=90.0;reference.grad_start=0.08;reference.grad_end=0.12;
    world.spline_edits.lock().changed.extend([(a,reference.clone()),(b,bad.clone())]);
    editor.selected=Some(b);editor.group=vec![a,b];
    let msg=editor.marking_to_road(&world);assert!(msg.starts_with("Marking fitted"),"{msg}");
    let fixed=world.spline_edits.lock().current(b).unwrap();
    assert_eq!(world.spline_edits.lock().current(a).unwrap(),reference);
    assert_eq!(fixed.length,reference.length);assert_eq!(fixed.radius,reference.radius);
    assert_eq!(fixed.prev_id,bad.prev_id);assert_eq!(fixed.next_id,bad.next_id);assert_eq!(fixed.file,bad.file);
    for i in 0..=20 {for x in [-3.94,0.0,3.94] {
        let station=reference.length*i as f64/20.0;
        let asphalt=omsi_geometry::spline_profile_point(&def,&curve(a.0,&reference),false,station,x,0.1);
        let marking=omsi_geometry::spline_profile_point(&md.def,&curve(b.0,&fixed),false,station,x,0.105);
        assert!((marking.z-asphalt.z-0.005).abs()<0.001,"paint clearance at {station}");
    }}
    editor.undo(&world);assert_eq!(world.spline_edits.lock().current(b).unwrap(),bad);
    // Reject cross-sections that a rigid spline profile cannot conform to.
    let mut raised=md.def.clone();
    raised.profiles[0].points[0].z+=0.1;
    assert!(aligned_marking(a,&reference,&def,b,&bad,&raised).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn marking_follows_entire_curved_chain_and_undo_restores_all_parts() {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root=std::env::temp_dir().join(format!("omsi-marking-chain-{stamp}"));let map=root.join("maps/Test");
    std::fs::create_dir_all(&map).unwrap();std::fs::create_dir_all(root.join("Splines")).unwrap();
    let profile=|a:f64,b:f64,z:f64|format!("[profile]\n0\n[profilepnt]\n{a}\n{z}\n0\n1\n[profilepnt]\n{b}\n{z}\n1\n1\n");
    std::fs::write(root.join("Splines/road.sli"),profile(-5.0,5.0,0.1)).unwrap();
    std::fs::write(root.join("Splines/paint.sli"),profile(-0.06,0.06,0.105)).unwrap();
    std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n").unwrap();
    for x in 0..=1 {std::fs::write(map.join(format!("tile_{x}_0.map")),"[version]\n14\n").unwrap();}
    let world=World::open(&root,&map.join("global.cfg"),20000101).unwrap();
    let mut roads=Vec::new();
    let mut road=MapSpline{id:101,file:"Splines/road.sli".into(),pos:[295.0,100.0,5.0],heading:90.0,length:5.0,radius:100.0,grad_start:1.0,grad_end:1.0,..Default::default()};
    let mut tile=(0,0);
    for i in 0..3 {
        road.id=101+i;road.prev_id=if i==0{0}else{100+i};road.next_id=if i==2{0}else{102+i};
        roads.push(((tile,road.id),road.clone()));
        let c=curve(tile,&road);let end=c.end_point();let next_tile=tile_at(end);let o=origin(next_tile);
        road.pos=[end.x-o.x,end.y-o.y,end.z];road.heading=c.heading_at(road.length);tile=next_tile;
    }
    let mk=((0,0),201);let mark=MapSpline{id:201,file:"Splines/paint.sli".into(),length:44.0,pos:[295.0,100.0,8.0],next_id:202,tex_offset:2.0,..Default::default()};
    let nk=((1,0),202);let neighbor=MapSpline{id:202,prev_id:201,file:mark.file.clone(),length:10.0,..Default::default()};
    world.spline_edits.lock().originals.extend(roads.clone());world.spline_edits.lock().originals.extend([(mk,mark.clone()),(nk,neighbor.clone())]);
    let mut editor=SplineEditor::default();editor.selected=Some(mk);
    editor.group=vec![roads[2].0,mk,roads[0].0,roads[1].0];let selection=editor.group.clone();
    let result=editor.marking_to_road(&world);assert!(result.starts_with("Marking fitted"),"{result}");
    let mut id=201;let mut generated=Vec::new();
    for (i,(_,road)) in roads.iter().enumerate() {
        let edits=world.spline_edits.lock();let k=edits.originals.keys().chain(edits.added.keys()).find(|k|k.1==id).copied().unwrap();
        let part=edits.current(k).unwrap();assert!(!part.deleted);assert_eq!(part.length,road.length);assert_eq!(part.radius,road.radius);
        assert!((part.tex_offset-(2.0+i as f64*5.0)).abs()<1e-8);
        let rc=curve(roads[i].0.0,road);let pc=curve(k.0,&part);
        for j in 0..=10 {assert!(rc.point_at(j as f64*0.5).distance(pc.point_at(j as f64*0.5))<1e-6);}
        id=part.next_id;generated.push((k,part));
    }
    assert_eq!(id,202);assert_eq!(world.spline_edits.lock().current(nk).unwrap().prev_id,generated[2].0.1);
    // Generated records retain the geometry and links through map serialization.
    let text=String::from("[version]\n14\n")+&generated.iter().map(|(_,s)|record(s,"0","\n")).collect::<String>();
    let check=map.join("roundtrip.map");std::fs::write(&check,text).unwrap();let loaded=Tile::load(&check).unwrap();
    assert_eq!(loaded.splines.len(),3);assert_eq!(loaded.splines[0].next_id,generated[1].0.1);
    editor.undo(&world);assert_eq!(world.spline_edits.lock().current(mk).unwrap(),mark);assert_eq!(world.spline_edits.lock().current(nk).unwrap(),neighbor);
    assert_eq!(editor.group,selection);
    for (k,_) in generated.iter().skip(1) {assert!(world.spline_edits.lock().current(*k).unwrap().deleted);}
    for (k,r) in &roads {assert_eq!(world.spline_edits.lock().current(*k).unwrap(),*r);}
    assert!(marking_chain_order(&[roads[0].clone(),roads[2].clone()]).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn layered_terrain_fit_crosses_tiles_preserves_offset_links_and_undo() {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let base=std::env::temp_dir().join(format!("omsi-layered-terrain-{stamp}"));let dir=base.join("original");
    let map=dir.join("maps/Test");std::fs::create_dir_all(&map).unwrap();
    std::fs::create_dir_all(dir.join("Splines")).unwrap();
    for (name,width) in [("road",5.0),("marking",0.1)] {
        std::fs::write(dir.join(format!("Splines/{name}.sli")),format!("[profile]\n0\n[profilepnt]\n-{width}\n0\n0\n1\n[profilepnt]\n{width}\n0\n1\n1\n")).unwrap();
    }
    std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n").unwrap();
    let a=((0,0),1);let b=((0,0),2);
    let road=MapSpline{id:1,file:"Splines/road.sli".into(),pos:[290.0,150.0,3.0],heading:90.0,length:30.0,next_id:3,..Default::default()};
    let marking=MapSpline{id:2,file:"Splines/marking.sli".into(),pos:[290.0,150.0,3.02],next_id:0,..road.clone()};
    let next=MapSpline{id:3,pos:[320.0,150.0,3.0],prev_id:1,next_id:0,..road.clone()};
    std::fs::write(map.join("tile_0_0.map"),format!("[version]\n14\n{}{}{}",record(&road,"0","\n"),record(&marking,"0","\n"),record(&next,"0","\n"))).unwrap();
    std::fs::write(map.join("tile_1_0.map"),"[version]\n14\n").unwrap();
    let world=World::open(&dir,&map.join("global.cfg"),20000101).unwrap();
    let mut terrain=omsi_map::Terrain::flat();terrain.heights.fill(10.0);
    for tile in [(0,0),(1,0)] {world.terrain_edits.lock().insert(tile,terrain.clone());}
    world.spline_edits.lock().originals.extend([(a,road.clone()),(b,marking.clone()),(((0,0),3),next.clone())]);
    let mut editor=SplineEditor{selected:Some(a),group:vec![b,a],..Default::default()};
    let message=editor.fit_terrain(&world);assert!(message.contains("fitted together"),"{message}");
    assert_eq!(editor.undo.len(),1);assert_eq!(editor.group.len(),2);assert_eq!(editor.fitted_group.len(),12);
    let (r,m)={let edits=world.spline_edits.lock();(edits.current(a).unwrap(),edits.current(b).unwrap())};
    assert!(r.pos[2]>9.0);assert!((m.pos[2]-r.pos[2]-0.02).abs()<1e-8);
    {let edits=world.spline_edits.lock();
        for i in 0..6 {let m=edits.current(editor.fitted_group[i]).unwrap();let r=edits.current(editor.fitted_group[6+i]).unwrap();
            assert!((m.pos[2]-r.pos[2]-0.02).abs()<1e-8);assert_eq!(r.length,m.length);
            assert!((curve((0,0),&m).end_point().z-curve((0,0),&r).end_point().z-0.02).abs()<1e-8);
        }
        assert_eq!(edits.current(((0,0),3)).unwrap().prev_id,editor.group[1].1);
    }
    let description=editor.describe(&world);assert!(description.contains("1.00 m")||description.contains("5.00 m"));assert!(description.contains("%"));
    let content=base.join("saved");crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&dir).unwrap();
    assert_eq!(Tile::load(&content.join("maps/Test/tile_0_0.map")).unwrap().splines.len(),13);
    editor.undo(&world);assert_eq!(editor.selection_keys(),vec![b,a]);
    assert_eq!(world.spline_edits.lock().current(a).unwrap(),road);
    assert_eq!(world.spline_edits.lock().current(b).unwrap(),marking);
    assert_eq!(world.spline_edits.lock().current(((0,0),3)).unwrap(),next);
    assert_eq!(world.terrain_edits.lock()[&(0,0)],terrain);
    // Terrain-to-road also accepts the pair, touching only its footprint.
    let msg=editor.terrain_to_road(&world);assert!(msg.contains("Terrain fitted"),"{msg}");
    let lowered=world.terrain_edits.lock()[&(0,0)].clone();
    assert_eq!(lowered.height_at(0,0),10.0);assert!(lowered.sample(295.0,150.0)<4.0);
    editor.undo(&world);assert_eq!(world.terrain_edits.lock()[&(0,0)],terrain);
    // A mismatched selection must not partially apply either layer.
    world.spline_edits.lock().changed.insert(b,MapSpline{heading:0.0,..marking});
    let n=editor.undo.len();assert!(editor.fit_terrain(&world).contains("nothing changed"));assert_eq!(editor.undo.len(),n);
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
#[ignore = "Requires the local user map; never saves"]
fn saved_cross_tile_deleted_neighbors_and_group_terrain_probe() {
    let root=std::path::PathBuf::from(std::env::var("EDITOR_PROBE_CONTENT").unwrap());
    let world=World::open(&root,&root.join("maps/Lemmental_Krefrath_Merge_Test/global.cfg"),20261017).unwrap();
    let owner=(-16,-11);let (tile,_)=world.editor_row_source(owner).unwrap();
    let keys=[(owner,9860260),(owner,9860261)];
    for key in keys {let s=tile.splines.iter().find(|s|s.id==key.1).unwrap().clone();world.spline_edits.lock().originals.insert(key,s);}
    assert!(deleted_spline_link_in_world(&world,owner,9860262));
    assert!(deleted_spline_link_in_world(&world,owner,9860263));
    for dx in -1..=1 {for dy in -1..=1 {let key=(owner.0+dx,owner.1+dy);
        if let Some(path)=world.tile_source(key.0,key.1) {let p=std::path::PathBuf::from(format!("{}.terrain",path.display()));
            if let Ok(t)=omsi_map::Terrain::load(&p) {world.terrain_edits.lock().insert(key,t);}
        }
    }}
    let mut editor=SplineEditor{selected:Some(keys[1]),group:keys.to_vec(),..Default::default()};
    let msg=editor.fit_terrain(&world);assert!(msg.contains("fitted together"),"{msg}");
    assert_eq!(editor.undo.len(),1);editor.undo(&world);
    assert!(editor.extend_group(&world).contains("gemeinsam fortgesetzt"));
}

#[test]
#[ignore = "Requires a local Merge map; reads it without saving or modifying it"]
fn saved_merge_map_read_only_continuation_probe() {
    let root=std::path::PathBuf::from(std::env::var("EDITOR_PROBE_CONTENT").expect("content root"));
    let map=root.join("maps/Lemmental_Krefrath_Merge_Test/global.cfg");
    let world=World::open(&root,&map,20261017).unwrap();
    let owner=(-16,-11);
    let (tile,_)=world.editor_row_source(owner).unwrap();
    let mut keys=Vec::new();
    for id in [706831,715278] {
        let s=tile.splines.iter().find(|s|s.id==id).unwrap().clone();
        assert!(!s.deleted);let key=(owner,id);keys.push(key);
        world.spline_edits.lock().originals.insert(key,s);
    }
    for id in [9860260,9860259] {assert!(deleted_spline_link_in_world(&world,owner,id),"{id}");}
    let mut editor=SplineEditor{selected:keys.last().copied(),group:keys,..Default::default()};
    assert!(editor.extend_group(&world).contains("gemeinsam fortgesetzt"));
    for action in [Action::Length(1.0),Action::Curvature(0.001),Action::Grade(0.5)] {
        let n=editor.undo.len();let msg=editor.shape_group(&world,&action);
        assert_eq!(editor.undo.len(),n+1,"{msg}");
    }
    for _ in 0..3 {
        assert!(editor.extend_group(&world).contains("gemeinsam fortgesetzt"));
        for action in [Action::Length(1.0),Action::Curvature(-0.001),Action::Grade(-0.5)] {
            let n=editor.undo.len();let msg=editor.shape_group(&world,&action);
            assert_eq!(editor.undo.len(),n+1,"{msg}");
        }
    }
    // Deliberately no Editor::save: all changes above are in-memory only.
}

#[test]
fn saved_deleted_neighbor_across_tile_boundary_is_free_but_live_override_is_not() {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root=std::env::temp_dir().join(format!("omsi-dead-neighbor-{stamp}"));let map=root.join("maps/Test");
    std::fs::create_dir_all(&map).unwrap();
    std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n").unwrap();
    let dead=MapSpline{id:42,deleted:true,length:20.0,file:"road.sli".into(),..Default::default()};
    for x in [0,1] {std::fs::write(map.join(format!("tile_{x}_0.map")),format!("[version]\n14\n{}",record(&dead,"0","\n"))).unwrap();}
    let world=World::open(&root,&map.join("global.cfg"),20000101).unwrap();
    assert!(deleted_spline_link_in_world(&world,(0,0),42));
    assert!(!deleted_spline_link_in_world(&world,(0,0),43));
    world.spline_edits.lock().changed.insert(((1,0),42),MapSpline{deleted:false,..dead.clone()});
    assert!(!deleted_spline_link_in_world(&world,(0,0),42));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deleted_link_requires_unique_known_deleted_record() {
    let key=((0,0),42);let dead=MapSpline{id:42,deleted:true,..Default::default()};
    let mut edits=Edits::default();
    assert!(!deleted_spline_link(&edits,42));
    edits.originals.insert(key,dead.clone());assert!(deleted_spline_link(&edits,42));
    edits.changed.insert(key,MapSpline{deleted:false,..dead.clone()});assert!(!deleted_spline_link(&edits,42));
    edits.changed.insert(key,dead.clone());edits.added.insert(key,dead.clone());assert!(deleted_spline_link(&edits,42));
    edits.originals.insert(((1,0),42),dead);assert!(!deleted_spline_link(&edits,42));
}

#[test]
fn group_shaping_rejects_mismatched_sections_connected_ends_and_boundary_crossing() {
    let s=MapSpline{id:1,pos:[100.0,100.0,0.0],length:20.0,..Default::default()};
    let mut parts=vec![(((0,0),1),s.clone()),(((0,0),2),MapSpline{id:2,pos:[103.0,101.0,0.0],..s.clone()})];
    assert!(shaped_group(&parts,&Action::Curvature(0.01)).is_err());
    parts[1].1.pos[1]=100.0;parts[1].1.next_id=9;
    assert!(shaped_group(&parts,&Action::Length(1.0)).is_err());
    parts[1].1.next_id=0;parts[1].1.prev_id=9;
    assert!(shaped_group(&parts,&Action::Turn(5.0)).is_err());
    assert!(shaped_group(&parts,&Action::Curvature(0.01)).is_ok());
    parts[1].1.prev_id=0;
    assert!(shaped_group(&parts,&Action::Length(400.0)).is_ok());
}

#[test]
fn additive_overlap_selection_keeps_group_and_stops_when_all_are_selected() {
    let a=((0,0),1); let b=((0,0),2); let c=((0,0),3);
    let mut editor=SplineEditor {selected:Some(a),group:vec![a],candidates:vec![a,b,c],next:1,..Default::default()};
    assert_eq!(editor.add_next_pick(),Some(2));
    assert_eq!(editor.selection_keys(),vec![a,b]);
    assert_eq!(editor.add_next_pick(),Some(3));
    assert_eq!(editor.selection_keys(),vec![a,b,c]);
    editor.add_next_pick();
    assert_eq!(editor.selection_keys(),vec![a,b,c]);
    editor.next_pick();
    assert_eq!(editor.selection_keys().len(),1);
}

#[test]
fn group_continuation_links_tile_cuts_preserves_marking_and_undoes_atomically() {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir=std::env::temp_dir().join(format!("omsi-group-continue-{stamp}"));
    let root=dir.join("original"); let map=root.join("maps/Test"); let content=dir.join("mod");
    std::fs::create_dir_all(&map).unwrap();
    std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n").unwrap();
    let a=((0,0),1);let b=((0,0),2);
    let road=MapSpline{id:1,next_id:41,file:"road.sli".into(),pos:[280.0,20.0,3.0],heading:90.0,length:10.0,grad_end:2.0,cant_end:3.0,..Default::default()};
    let marking=MapSpline{id:2,next_id:42,file:"marking.sli".into(),pos:[270.0,22.0,3.01],length:20.0,mirror:true,tex_offset:17.0,..road.clone()};
    std::fs::write(map.join("tile_0_0.map"),format!("[version]\n14\n{}{}",record(&road,"0","\n"),record(&marking,"0","\n"))).unwrap();
    std::fs::write(map.join("tile_1_0.map"),"[version]\n14\n").unwrap();
    let world=World::open(&root,&map.join("global.cfg"),20000101).unwrap();
    world.spline_edits.lock().originals.extend([(a,road.clone()),(b,marking.clone()),
        (((0,0),41),MapSpline{id:41,deleted:true,..Default::default()}),
        (((0,0),42),MapSpline{id:42,deleted:true,..Default::default()})]);
    let mut editor=SplineEditor{selected:Some(b),group:vec![a,b],..Default::default()};
    let message=editor.apply(&world,&Action::Copy).unwrap();
    assert!(message.contains("gemeinsam fortgesetzt"),"{message}");
    assert_eq!(editor.undo.len(),1); assert_eq!(editor.group.len(),2);
    {
        let edits=world.spline_edits.lock();
        assert_eq!(edits.added.len(),2);
        for (key,old) in [(a,&road),(b,&marking)] {
            let first=edits.added.values().find(|s|s.prev_id==old.id).unwrap();

            assert_eq!(edits.current(key).unwrap().next_id,first.id);
            assert_eq!(first.next_id,0);
            assert!((first.length-20.0).abs()<1e-8);
            assert_eq!(first.mirror,old.mirror); assert_eq!(first.cant_start,old.cant_end);
            assert!((first.tex_offset-old.tex_offset-old.length).abs()<1e-8);

            assert!(curve(key.0,old).end_point().distance(curve((0,0),first).point_at(0.0))<1e-8);
            assert!(curve((0,0),first).end_point().x>300.0);
        }
    }
    crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&root).unwrap();
    let saved=Tile::load(&content.join("maps/Test/tile_0_0.map")).unwrap();
    assert_eq!(saved.splines.len(),4);
    // Shaping updates every selected member and is also one undo transaction.
    let selected=editor.selection_keys();
    let snapshot=selected.iter().map(|k|(*k,world.spline_edits.lock().current(*k).unwrap())).collect::<Vec<_>>();
    editor.apply(&world,&Action::Curvature(0.01)).unwrap();
    assert_eq!(editor.undo.len(),2);
    for key in &selected {assert_ne!(world.spline_edits.lock().current(*key).unwrap().radius,0.0);}
    crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&root).unwrap();
    for (key,_) in &snapshot {
        let saved=Tile::load(&content.join(format!("maps/Test/tile_{}_{}.map",key.0.0,key.0.1))).unwrap();
        let current=world.spline_edits.lock().current(*key).unwrap();
        let stored=saved.splines.iter().find(|s|s.id==key.1).unwrap();
        assert!((stored.radius-current.radius).abs()<1e-8);
        assert!((stored.length-current.length).abs()<1e-8);
    }
    editor.undo(&world);
    assert_eq!(editor.selection_keys(),selected);
    for (key,old) in snapshot {assert_eq!(world.spline_edits.lock().current(key).unwrap(),old);}
    // Repeated continuation has its own undo step and restores the preceding group.
    let previous_group=editor.selection_keys();
    assert!(editor.apply(&world,&Action::Copy).unwrap().contains("gemeinsam fortgesetzt"));
    assert_eq!(editor.undo.len(),2);
    editor.undo(&world);
    assert_eq!(editor.selection_keys(),previous_group);
    editor.undo(&world);
    assert_eq!(editor.selection_keys(),vec![a,b]);
    {
        let edits=world.spline_edits.lock();
        assert_eq!(edits.current(a).unwrap(),road); assert_eq!(edits.current(b).unwrap(),marking);
        assert!(edits.added.values().all(|s|s.deleted));
    }
    // A blocked member must not partially extend the other member.
    let mut blocked=marking.clone(); blocked.next_id=999;
    world.spline_edits.lock().changed.insert(b,blocked);
    editor.group=vec![a,b];editor.selected=Some(a);
    assert!(editor.apply(&world,&Action::Copy).unwrap().contains("nichts geändert"));
    assert_eq!(editor.undo.len(),0);
    assert_eq!(world.spline_edits.lock().current(a).unwrap(),road);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn group_move_drag_undo_boundary_rejection_and_save_cover_multiple_tiles() {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir=std::env::temp_dir().join(format!("omsi-spline-group-{stamp}"));
    let original=dir.join("original");let map=original.join("maps/Test");let content=dir.join("mod");
    std::fs::create_dir_all(&map).unwrap();
    std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n").unwrap();
    let a=((0,0),1);let b=((1,0),2);
    let first=MapSpline{id:1,pos:[100.0,20.0,3.0],length:20.0,file:"road.sli".into(),..Default::default()};
    let second=MapSpline{id:2,pos:[110.0,30.0,4.0],..first.clone()};
    for (key,spline) in [(a,&first),(b,&second)] {
        std::fs::write(map.join(format!("tile_{}_0.map",key.0.0)),format!("[version]\n14\n{}",record(spline,"0","\n"))).unwrap();
    }
    let world=World::open(&original,&map.join("global.cfg"),20000101).unwrap();
    world.spline_edits.lock().originals.extend([(a,first.clone()),(b,second.clone())]);
    let mut editor=SplineEditor{selected:Some(a),group:vec![a,b],..Default::default()};
    editor.apply(&world,&Action::Move(DVec3::new(2.0,3.0,4.0))).unwrap();
    assert_eq!(world.spline_edits.lock().current(a).unwrap().pos,[102.0,23.0,7.0]);
    assert_eq!(world.spline_edits.lock().current(b).unwrap().pos,[112.0,33.0,8.0]);
    assert_eq!(world.spline_edits.lock().dirty_tiles.len(),2);
    assert_eq!(editor.undo.len(),1);editor.undo(&world);
    assert_eq!(world.spline_edits.lock().current(a).unwrap(),first);
    assert_eq!(world.spline_edits.lock().current(b).unwrap(),second);
    let undo_count=editor.undo.len();
    assert!(editor.move_group(&world,DVec3::new(1000.0,0.0,0.0)).unwrap().contains("nothing moved"));
    assert_eq!(editor.undo.len(),undo_count);
    assert_eq!(world.spline_edits.lock().current(a).unwrap(),first);
    assert_eq!(world.spline_edits.lock().current(b).unwrap(),second);
    let ground=curve(a.0,&first).point_at(0.0);
    editor.begin_drag(&world,ground);
    editor.drag_to(&world,ground+DVec3::new(2.0,0.0,0.0)).unwrap();
    editor.drag_to(&world,ground+DVec3::new(5.0,0.0,0.0)).unwrap();
    editor.finish_drag();assert_eq!(editor.undo.len(),undo_count+1);
    assert_eq!(world.spline_edits.lock().current(b).unwrap().pos[0],115.0);
    crate::editor::Editor::default().save(&world,"maps/Test/global.cfg",&content,&original).unwrap();
    for (key,old) in [(a,&first),(b,&second)] {
        let saved=Tile::load(&content.join(format!("maps/Test/tile_{}_0.map",key.0.0))).unwrap();
        assert_eq!(saved.splines[0].pos[0],old.pos[0]+5.0);
        assert_eq!(saved.splines[0].id,old.id);
        assert_eq!(Tile::load(&map.join(format!("tile_{}_0.map",key.0.0))).unwrap().splines[0].pos[0],old.pos[0]);
    }
    editor.undo(&world);
    assert_eq!(world.spline_edits.lock().current(a).unwrap(),first);
    assert_eq!(world.spline_edits.lock().current(b).unwrap(),second);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn selection_categories_prefer_paths_and_keep_unknown_assets_accessible() {
    let mut def = omsi_scenery::Spline::default();
    assert_eq!(selection_category("Splines/Markierungen/road.sli", Some(&def)), SelectionFilter::Other);
    assert_eq!(selection_category("Splines/markierung_weiss.sli", Some(&def)), SelectionFilter::Markings);
    assert_eq!(selection_category("Splines/Marktplatz.sli", Some(&def)), SelectionFilter::Other);
    for name in ["BS_StrMarkierung_WeissGestrichelt_M.sli", "BS_StrMarkierung_Weiss_S.sli", "BS_StrMarkierung_WeissGestrichelt_S.sli"] {
        assert_eq!(selection_category(name, Some(&def)), SelectionFilter::Markings);
    }
    assert_eq!(selection_category("Splines/StrMarkierung/terrain.sli", Some(&def)), SelectionFilter::Other);
    def.paths.push(omsi_scenery::PathDef { kind: 1, ..Default::default() });
    assert_eq!(selection_category("road.sli", Some(&def)), SelectionFilter::Sidewalks);
    def.paths.push(omsi_scenery::PathDef { kind: 0, ..Default::default() });
    assert_eq!(selection_category("markierung_road.sli", Some(&def)), SelectionFilter::Roads);
    assert_eq!(selection_category("road.sli", None), SelectionFilter::Other);
    for category in [SelectionFilter::Roads, SelectionFilter::Sidewalks, SelectionFilter::Markings, SelectionFilter::Other] {
        assert!(SelectionFilter::All.accepts(category));
    }
}

#[test]
fn filtered_pick_and_tab_exclude_overlaid_marking_without_changing_records() {
    let keys = [((0,0),1), ((0,0),2)];
    let road = between("road.sli",1,(0,0),DVec3::new(100.0,100.0,0.0),DVec3::new(100.0,120.0,0.0),0.0);
    let mut marking = road.clone(); marking.id=2; marking.file="markierung.sli".into();
    let cache = parking_lot::Mutex::new(Edits { originals: HashMap::from([(keys[0],road),(keys[1],marking)]), ..Default::default() });
    let mut editor=SplineEditor::default();
    editor.selected=Some(keys[1]); editor.candidates=keys.to_vec();
    editor.cycle_selection_filter();
    assert_eq!(editor.selected,Some(keys[1])); assert!(editor.candidates.is_empty());
    let edges=HashMap::from([(keys[0],Vec::new())]);
    assert_eq!(editor.pick_loaded(&cache,&[(0,0)],DVec3::new(100.0,110.0,10.0),-Vec3::Z,&edges),Some(1));
    assert_eq!(editor.next_pick(),Some(1));
    assert!(!cache.lock().dirty);
    editor.selection_filter=SelectionFilter::All;
    editor.pick_loaded(&cache,&[(0,0)],DVec3::new(100.0,110.0,10.0),-Vec3::Z,&edges);
    assert_eq!(editor.candidates.len(),2);
    editor.selection_filter=SelectionFilter::Other;
    assert_eq!(editor.pick_loaded(&cache,&[(0,0)],DVec3::new(100.0,110.0,10.0),-Vec3::Z,&HashMap::new()),None);
}

#[test]
fn second_connection_pick_releases_the_cache_before_preview_reads_it() {
    let source_key = ((0, 0), 1);
    let target_key = ((0, 0), 2);
    let source = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 0.0), DVec3::new(100.0, 110.0, 0.0), 0.0);
    let target = between("road.sli", 2, (0, 0), DVec3::new(100.0, 111.0, 0.0), DVec3::new(100.0, 120.0, 0.0), 0.0);
    let cache = parking_lot::Mutex::new(Edits { originals: HashMap::from([(source_key, source), (target_key, target)]), ..Default::default() });
    let mut editor = SplineEditor::default();
    assert_eq!(editor.pick_loaded(&cache, &[(0, 0)], DVec3::new(100.0, 100.0, 10.0), -Vec3::Z, &HashMap::new()), Some(1));
    assert!(cache.try_lock().is_some());
    editor.connection = Some(Connection { source: source_key, target: None, preview: None, error: None, replace_links: false, occupied: false, allow_transition: false });
    assert_eq!(editor.pick_loaded(&cache, &[(0, 0)], DVec3::new(100.0, 120.0, 10.0), -Vec3::Z, &HashMap::new()), Some(2));
    assert_eq!(editor.connection.as_ref().unwrap().source, source_key);
    // try_lock makes a lock-lifetime regression fail immediately rather than
    // hanging the test runner, just as the old second-click path hung the UI.
    let (source, target) = {
        let edits = cache.try_lock().expect("second pick retained the spline cache lock");
        (edits.current(source_key).unwrap(), edits.current(target_key).unwrap())
    };
    assert!(connection_geometry((0, 0), &source, (0, 0), &target).is_ok());
    assert!(cache.try_lock().is_some());
}

#[test]
fn generated_spline_roundtrips_axes_and_height() {
    let start = DVec3::new(223.5745, 269.2551, 42.26);
    let end = DVec3::new(240.0, 255.0, 41.36);
    let s = between("Splines\\road.sli", 9852695, (0, 0), start, end, 0.0);
    let tile = parsed(&record(&s, "0", "\r\n"));
    assert_eq!(tile.splines.len(), 1);
    let r = &tile.splines[0];
    assert!((curve((0, 0), r).end_point() - end).length() < 1e-8);
    assert_eq!(r.pos, s.pos);
    assert!((r.delta_h.unwrap() + 0.9).abs() < 1e-10);
    assert_eq!(record(&s, "0", "\n").lines().count(), 20);
}

#[test]
fn generated_ai_rules_survive_map_save_and_reload() {
    let settings=crate::traffic_editor::Settings {forward_bus:true,..Default::default()};
    let source=MapSpline {id:1,length:20.0,cant_start:3.0,cant_end:7.0,skew_start:0.2,skew_end:-0.1,profile_transitions:[None,Some(omsi_map::ProfileTransition {station:20.0,span:5.0,x:[-4.0,4.0],offsets:[[0.5,-0.05,-0.1],[-0.5,0.05,-0.1]]})],..Default::default()};
    let mut plan=crate::traffic_editor::build(&[(((0,0),1),source,false)],&settings).unwrap();
    let s=&mut plan.pieces[0].1;s.id=2;
    let edits=HashMap::from([(2,s.clone())]);
    let (text,n)=rewrite("[version]\n14\n",&edits,&[2]).unwrap();assert_eq!(n,1);
    let tile=parsed(&text);assert_eq!(tile.splines.len(),1);
    assert_eq!(tile.splines[0].rules,s.rules);
    assert_eq!(tile.splines[0].cant_start,s.cant_start);assert_eq!(tile.splines[0].cant_end,s.cant_end);
    assert_eq!(tile.splines[0].skew_start,s.skew_start);assert_eq!(tile.splines[0].skew_end,s.skew_end);
    assert_eq!(tile.splines[0].profile_transitions,s.profile_transitions);
    assert_eq!(rewrite(&text,&edits,&[2]).unwrap().0,text);
}

#[test]
fn rewrites_preserve_order_rules_and_are_idempotent() {
    let a = between("Splines\\road.sli", 1, (0, 0), DVec3::new(10.0, 20.0, 5.0), DVec3::new(10.0, 50.0, 6.0), 0.0);
    let mut b = a.clone(); b.id = 2; b.is_h = false; b.delta_h = None;
    let text = format!("[version]\r\n14\r\n\r\n{}\r\n[rule]\r\n0\r\nspeedlimit\r\n30\r\n0\r\n\r\n{}\r\n[terrain]\r\n", record(&a, "2", "\r\n"), record(&b, "0", "\r\n"));
    let mut changed = a.clone(); changed.pos[0] += 1.0;
    let mut new = a.clone(); new.id = 3;
    let edits = HashMap::from([(1, changed), (3, new)]);
    let (out, n) = rewrite(&text, &edits, &[3]).unwrap();
    assert_eq!(n, 2);
    assert!(out.contains("[rule]\r\n0\r\nspeedlimit\r\n30\r\n0\r\n"));
    assert!(out.contains(&record(&b, "0", "\r\n")));
    let t = parsed(&out);
    assert_eq!(t.splines.iter().map(|s| s.id).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(t.splines[0].rules.len(), 1);
    assert_eq!(rewrite(&out, &edits, &[3]).unwrap().0, out);
}

#[test]
fn missing_ids_and_old_tiles_are_rejected() {
    let s = between("Splines\\road.sli", 9, (0, 0), DVec3::new(1.0, 1.0, 0.0), DVec3::new(1.0, 5.0, 0.0), 0.0);
    let e = HashMap::from([(9, s)]);
    assert!(rewrite("[version]\n14\n", &e, &[]).is_err());
    assert!(rewrite("[version]\n11\n", &e, &[9]).is_err());
    let (out, _) = rewrite("[version]\n14\n", &e, &[9]).unwrap();
    assert_eq!(parsed(&out).splines[0].id, 9);
}

#[test]
fn overlay_keeps_attachment_indices_and_never_appends_twice() {
    let s = between("Splines\\road.sli", 9, (0, 0), DVec3::new(1.0, 1.0, 0.0), DVec3::new(1.0, 5.0, 0.0), 0.0);
    let mut e = Edits::default();
    e.added.insert(((0, 0), 9), s);
    let mut t = Tile::default();
    e.overlay((0, 0), &mut t); e.overlay((0, 0), &mut t);
    assert_eq!(t.splines.len(), 1);
}

#[test]
fn deletion_and_restoration_keep_indices_rules_and_line_endings() {
    let mut a = between("road.sli", 11, (0, 0), DVec3::new(10.0, 10.0, 2.0), DVec3::new(10.0, 40.0, 3.0), 0.0);
    let mut b = a.clone(); b.id = 12;
    let text = format!("[version]\r\n14\r\n{}[rule]\r\n0\r\nspeedlimit\r\n30\r\n0\r\n{}[splineAttachement]\r\n0\r\nlamp.sco\r\n50\r\n1\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n10\r\n100\r\n0\r\n0\r\n", record(&a, "0", "\r\n"), record(&b, "0", "\r\n"));
    a.deleted = true;
    let edits = HashMap::from([(11, a.clone())]);
    let deleted = rewrite(&text, &edits, &[]).unwrap().0;
    let tile = parsed(&deleted);
    assert_eq!(tile.splines.iter().map(|s| s.id).collect::<Vec<_>>(), vec![11, 12]);
    assert!(tile.splines[0].deleted && !tile.splines[1].deleted);
    assert_eq!(tile.splines[0].rules.len(), 1);
    assert_eq!(tile.spline_attachments[0].spline_index, 1);
    assert_eq!(rewrite(&deleted, &edits, &[]).unwrap().0, deleted);
    a.deleted = false;
    assert_eq!(rewrite(&deleted, &HashMap::from([(11, a)]), &[]).unwrap().0, text);
}

#[test]
fn subdividing_a_curve_preserves_its_shape_height_and_texture_distance() {
    let mut s = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 2.0), DVec3::new(100.0, 160.0, 8.0), 12.0);
    s.radius = -80.0; s.grad_start = 3.0; s.grad_end = 7.0;
    let c = curve((0, 0), &s);
    let part = curve_part(&s, 23.0, 37.0);
    let p = curve((0, 0), &part);
    for i in 0..=20 {
        let d = part.length * i as f64 / 20.0;
        assert!((p.point_at(d) - c.point_at(d + 23.0)).length() < 1e-8);
        assert!((p.slope_at(d) - c.slope_at(d + 23.0)).abs() < 1e-10);
    }
    assert_eq!(part.tex_offset, 35.0);
}

#[test]
fn terrain_fit_follows_a_hill_without_overshooting_between_samples() {
    let mut s = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 0.0), DVec3::new(100.0, 120.0, 0.0), 0.0);
    s.radius = 60.0;
    let heights = [0.05, 1.05, 2.05, 1.05, 0.05];
    let parts = terrain_parts(&s, &heights);
    let original = curve((0, 0), &s);
    for (i, part) in parts.iter().enumerate() {
        let c = curve((0, 0), part);
        assert!((c.start.z - heights[i]).abs() < 1e-10);
        assert!((c.end_point().z - heights[i + 1]).abs() < 1e-10);
        for n in 0..=20 {
            let d = part.length * n as f64 / 20.0;
            let p = c.point_at(d);
            assert!(p.z >= heights[i].min(heights[i + 1]) - 1e-9 && p.z <= heights[i].max(heights[i + 1]) + 1e-9);
            assert!((p.truncate() - original.point_at(i as f64 * 5.0 + d).truncate()).length() < 1e-8);
        }
    }
}

#[test]
fn snap_arc_hits_targets_on_both_sides_and_on_a_straight() {
    let s = between("road.sli", 1, (0, 0), DVec3::new(100.0, 100.0, 2.0), DVec3::new(100.0, 110.0, 2.0), 0.0);
    for target in [DVec3::new(110.0, 140.0, 7.0), DVec3::new(90.0, 140.0, -1.0), DVec3::new(100.0, 140.0, 2.0)] {
        let snapped = arc_to(&s, (0, 0), target).unwrap();
        assert!((curve((0, 0), &snapped).end_point() - target).length() < 1e-8);
        assert_eq!(snapped.pos, s.pos);
        assert_eq!(snapped.heading, s.heading);
    }
}

#[test]
fn generated_lines_split_at_grid_edges_in_negative_coordinates() {
    let size = omsi_map::tile_size();
    let start = DVec3::new(-size - 10.0, 50.0, 1.0);
    let end = DVec3::new(-size + 30.0, 50.0, 3.0);
    let cuts = straight_tile_cuts(start, end);
    assert_eq!(cuts, vec![0.0, 0.25, 1.0]);
    for pair in cuts.windows(2) {
        let a = start.lerp(end, pair[0]);
        let b = start.lerp(end, pair[1]);
        let tile = tile_at((a + b) * 0.5);
        let part = between("road.sli", 1, tile, a, b, 0.0);
        assert!(inside_tile(&part));
        assert!((curve(tile, &part).end_point() - b).length() < 1e-8);
    }
}

#[test]
fn connects_the_start_of_a_short_spline_across_a_tile_edge() {
    let size = omsi_map::tile_size();
    let source_tile = (1, 0);
    let target_tile = (0, 0);
    let source = between("terrain.sli", 239, source_tile, DVec3::new(size + 0.4, 80.0, 0.05), DVec3::new(size + 7.34, 80.0, 0.05), 0.0);
    let target = between("terrain.sli", 238, target_tile, DVec3::new(size - 20.0, 80.0, 0.05), DVec3::new(size - 0.2, 80.0, 0.05), 10.0);
    let joined = connection_geometry(source_tile, &source, target_tile, &target).unwrap();
    assert_eq!((joined.source_end, joined.target_end), (End::Start, End::Finish));
    assert!((curve(source_tile, &joined.source).start - curve(target_tile, &target).end_point()).length() < 1e-8);
    assert!((curve(source_tile, &joined.source).end_point() - curve(source_tile, &source).end_point()).length() < 1e-8);
    assert!((joined.source.length - 7.54).abs() < 1e-8);
    assert!(joined.source.pos[0] < 0.0 && !inside_tile(&joined.source));
    assert_eq!((joined.source.prev_id, joined.target.next_id), (238, 239));
    assert_eq!(joined.source.id, source.id);
    let tiles = connection_tiles((source_tile, source.id), (target_tile, target.id), &joined.source);
    assert!(tiles.contains(&source_tile) && tiles.contains(&target_tile));
    // Save the out-of-tile start without moving its record or attachment index.
    let base = format!("[version]\n14\n{}", record(&source, "0", "\n"));
    let edits = HashMap::from([(source.id, joined.source.clone())]);
    let written = rewrite(&base, &edits, &[]).unwrap().0;
    let read = parsed(&written);
    assert_eq!(read.splines.len(), 1);
    assert!((curve(source_tile, &read.splines[0]).start - curve(target_tile, &target).end_point()).length() < 1e-8);
    assert_eq!(rewrite(&written, &edits, &[]).unwrap().0, written);
}

#[test]
fn connects_all_endpoint_orientations_and_preserves_the_far_anchor() {
    let source = between("road.sli", 1, (0, 0), DVec3::new(50.0, 50.0, 2.0), DVec3::new(50.0, 60.0, 2.2), 0.0);
    let targets = [
        (End::Finish, End::Start, DVec3::new(50.0, 60.3, 2.3), DVec3::new(50.0, 80.0, 3.0)),
        (End::Finish, End::Finish, DVec3::new(50.0, 80.0, 3.0), DVec3::new(50.0, 60.3, 2.3)),
        (End::Start, End::Finish, DVec3::new(50.0, 30.0, 1.0), DVec3::new(50.0, 49.7, 1.8)),
        (End::Start, End::Start, DVec3::new(50.0, 49.7, 1.8), DVec3::new(50.0, 30.0, 1.0)),
    ];
    for (source_end, target_end, from, to) in targets {
        let target = between("road.sli", 2, (0, 0), from, to, 10.0);
        let joined = connection_geometry((0, 0), &source, (0, 0), &target).unwrap();
        assert_eq!((joined.source_end, joined.target_end), (source_end, target_end));
        let c = curve((0, 0), &joined.source);
        assert!((source_end.point(&c) - target_end.point(&curve((0, 0), &target))).length() < 1e-8);
        let fixed = if source_end == End::Start { End::Finish } else { End::Start };
        assert!((fixed.point(&c) - fixed.point(&curve((0, 0), &source))).length() < 1e-8);
        assert_eq!(source_end.link(&joined.source), 2);
        assert_eq!(target_end.link(&joined.target), 1);
        assert_eq!(joined.target.pos, target.pos);
        assert_eq!(joined.target.heading, target.heading);
    }
}

#[test]
fn connection_rejects_occupied_ends_and_bad_direction_without_changing_records() {
    let mut source = between("road.sli", 1, (0, 0), DVec3::new(50.0, 50.0, 0.0), DVec3::new(50.0, 60.0, 0.0), 0.0);
    let target = between("road.sli", 2, (0, 0), DVec3::new(50.0, 60.3, 0.0), DVec3::new(50.0, 80.0, 0.0), 0.0);
    source.next_id = 99;
    assert!(connection_geometry((0, 0), &source, (0, 0), &target).unwrap_err().contains("occupied"));
    source.next_id = 0;
    let side = between("road.sli", 3, (0, 0), DVec3::new(50.0, 60.3, 0.0), DVec3::new(70.0, 60.3, 0.0), 0.0);
    assert!(connection_geometry((0, 0), &source, (0, 0), &side).unwrap_err().contains("End directions"));
    assert_eq!(source.next_id, 0);
    assert_eq!(target.prev_id, 0);
}

fn road_patch(x0: f64, x1: f64, y0: f64, y1: f64, height: f64, grade: f64, cant: f64) -> Roadbed {
    let z = |x: f64, y: f64| height + (y - y0) * grade + (x - x0) * cant;
    let mesh = omsi_geometry::MeshData {
        positions: [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| DVec3::new(x, y, z(x, y)).as_vec3()).to_vec(),
        indices: vec![0, 1, 2, 0, 2, 3], ..Default::default()
    };
    let mut bed = Roadbed::default(); bed.add_mesh(&mesh, DVec3::ZERO); bed
}

#[test]
fn terrain_to_road_only_changes_samples_inside_surface_and_preserves_undo() {
    let bed = road_patch(142.0, 150.0, 50.0, 90.0, 3.0, 0.04, -0.02);
    for initial in [-5.0, 12.0] {
        let mut terrain = omsi_map::Terrain::flat(); terrain.heights.fill(initial);
        let original = HashMap::from([((0,0), terrain.clone())]);
        let plan = terrain_plan(&bed, &original, 0.0, 0.0);
        assert_eq!(plan.before, vec![((0,0), terrain.clone())]);
        let result = &plan.after[0].1;
        let cell = omsi_map::tile_size() / terrain.cells as f64;
        let mut changed = 0;
        for y in 0..terrain.samples() { for x in 0..terrain.samples() {
            let index = y * terrain.samples() + x;
            let p = DVec2::new(x as f64 * cell, y as f64 * cell);
            if let Some((_, height)) = bed.nearest(p, 0.0) {
                assert!((result.heights[index] as f64 - (height - ROAD_CLEARANCE)).abs() < 1e-5);
                changed += 1;
            } else { assert_eq!(result.heights[index], terrain.heights[index]); }
            assert!(result.heights[index].is_finite());
        }}
        assert!(changed > 0);
        assert!(terrain_plan(&bed,&HashMap::from([((0,0), result.clone())]),0.0,0.0).after.is_empty());
    }
}

#[test]
fn terrain_to_road_crosses_tile_boundary_and_undo_restores_both_tiles() {
    let size = omsi_map::tile_size();
    let bed = road_patch(size-12.0,size+12.0,40.0,90.0,2.0,0.03,0.01);
    let mut terrain = omsi_map::Terrain::flat(); terrain.heights.fill(9.0);
    let original = HashMap::from([((0,0),terrain.clone()),((1,0),terrain)]);
    let guard=size/original[&(0,0)].cells as f64*std::f64::consts::SQRT_2;
    let plan = terrain_plan(&bed,&original,guard,0.0);
    assert_eq!(plan.after.len(),2);
    let mut after: HashMap<_,_> = plan.after.iter().cloned().collect();
    let left=&after[&(0,0)]; let right=&after[&(1,0)];
    for y in 0..left.samples() {
        assert_eq!(left.height_at(left.samples()-1,y),right.height_at(0,y));
    }
    assert_eq!(left.height_at(0,0),9.0);
    assert_eq!(right.height_at(right.samples()-1,0),9.0);
    let undo=Undo{selection:None,splines:Vec::new(),terrain:plan.before,terrain_after:plan.after};
    undo.restore_terrain(&mut after); assert_eq!(after,original);
}

#[test]
fn narrow_road_without_grid_samples_does_not_modify_surrounding_terrain() {
    let bed=road_patch(147.0,147.1,51.0,89.0,0.2,0.0,0.0);
    let mut terrain=omsi_map::Terrain::flat();terrain.heights.fill(8.0);
    let plan=terrain_plan(&bed,&HashMap::from([((0,0),terrain)]),0.0,0.0);
    assert!(plan.after.is_empty());
}

#[test]
fn terrain_support_fills_full_width_between_raster_points() {
    for (x0,x1) in [(142.0,150.0),(147.0,147.1)] {
        let bed=road_patch(x0,x1,51.0,89.0,3.0,0.04,-0.02);
        for initial in [-8.0,12.0] {
            let mut terrain=omsi_map::Terrain::flat();terrain.heights.fill(initial);
            let guard=omsi_map::tile_size()/terrain.cells as f64*std::f64::consts::SQRT_2;
            let before=HashMap::from([((0,0),terrain.clone())]);let plan=terrain_plan(&bed,&before,guard,0.0);
            assert_eq!(plan.after.len(),1);let t=&plan.after[0].1;
            for x in [x0+0.001,(x0+x1)*0.5,x1-0.001] {for y in [51.01,55.0,67.3,88.99] {
                let road=3.0+(y-51.0)*0.04+(x-x0)*-0.02;
                assert!((t.sample(x as f32,y as f32) as f64-(road-ROAD_CLEARANCE)).abs()<0.002,"gap at {x},{y}");
            }}
            assert_eq!(t.height_at(0,0),initial);
            let mut restored=plan.after.iter().cloned().collect();
            Undo{selection:None,splines:Vec::new(),terrain:plan.before,terrain_after:plan.after}.restore_terrain(&mut restored);
            assert_eq!(restored,before);
        }
    }
}

#[test]
fn smoothing_clears_both_road_edges_and_leaves_distant_ground_unchanged() {
    let bed = road_patch(142.0, 150.0, 50.0, 90.0, 0.2, 0.015, -0.02);
    let mut terrain = omsi_map::Terrain::flat(); terrain.heights.fill(6.0);
    let original = HashMap::from([((0, 0), terrain.clone())]);
    let guard = omsi_map::tile_size() / terrain.cells as f64 * std::f64::consts::SQRT_2;
    let plan = terrain_plan(&bed, &original, guard, 2.0);
    assert_eq!(plan.after.len(), 1);
    let t = &plan.after[0].1;
    for x in [142.001, 145.0, 149.999] {
        for y in [50.01, 55.0, 69.3, 89.99] {
            let road = 0.2 + (y - 50.0) * 0.015 - (x - 142.0) * 0.02;
            assert!(t.sample(x as f32, y as f32) as f64 <= road - ROAD_CLEARANCE + 0.001);
        }
    }
    assert_eq!(t.height_at(0, 0), terrain.height_at(0, 0));
    assert_eq!(original[&(0, 0)], terrain);
    assert_eq!(plan.before[0].1, terrain);
}

#[test]
fn fitting_a_raised_profile_does_not_add_its_height_on_every_fit() {
    let profile = road_patch(140.0, 150.0, 50.0, 90.0, 1.2, 0.0, 0.0);
    let fitted = fitted_height(0.15, DVec3::new(145.0, 70.0, 1.0), &profile);
    assert!(fitted.abs() < 1e-6);
    let after = road_patch(140.0, 150.0, 50.0, 90.0, fitted + 0.2, 0.0, 0.0);
    let repeated = fitted_height(0.15, DVec3::new(145.0, 70.0, fitted), &after);
    assert!((repeated - fitted).abs() < 1e-6);
}

#[test]
fn narrow_road_clears_terrain_triangles_even_without_a_vertex_inside_its_width() {
    let bed = road_patch(147.0, 148.0, 51.0, 89.0, 0.2, 0.0, 0.0);
    let mut terrain = omsi_map::Terrain::flat(); terrain.heights.fill(8.0);
    let plan = terrain_plan(&bed, &HashMap::from([((0, 0), terrain)]), 0.0, 2.0);
    let t = &plan.after[0].1;
    for x in [147.001, 147.5, 147.999] {
        for y in [51.001, 52.4, 62.7, 75.9, 88.999] {
            assert!(t.sample(x, y) <= 0.151, "ground still covers the road at {x}/{y}");
        }
    }
}

#[test]
fn road_smoothing_matches_changed_nodes_on_both_sides_of_a_tile_border() {
    let size = omsi_map::tile_size();
    let bed = road_patch(size - 8.0, size + 8.0, 50.0, 90.0, 0.2, 0.01, 0.0);
    let mut a = omsi_map::Terrain::flat(); a.heights.fill(5.0);
    let mut b = a.clone(); b.heights.fill(6.0);
    let originals = HashMap::from([((0, 0), a.clone()), ((1, 0), b.clone())]);
    let plan = terrain_plan(&bed, &originals, size / a.cells as f64 * std::f64::consts::SQRT_2, 2.0);
    let changed: HashMap<_, _> = plan.after.into_iter().collect();
    for iy in 0..a.samples() {
        let left = changed[&(0, 0)].height_at(a.cells, iy);
        let right = changed[&(1, 0)].height_at(0, iy);
        if left != a.height_at(a.cells, iy) || right != b.height_at(0, iy) { assert_eq!(left, right); }
    }
    assert_eq!(changed[&(0, 0)].height_at(0, 0), 5.0);
    assert_eq!(changed[&(1, 0)].height_at(b.cells, 0), 6.0);
}

#[test]
fn smoothing_undo_restores_original_terrain_even_after_edited_bytes_were_saved() {
    let bed = road_patch(142.0, 150.0, 50.0, 90.0, 0.2, 0.0, 0.0);
    let mut t = omsi_map::Terrain::flat(); t.heights.fill(6.0);
    let plan = terrain_plan(&bed, &HashMap::from([((0, 0), t.clone())]), 8.0, 2.0);
    let mut editor = SplineEditor::default(); editor.remember_with_terrain(Vec::new(), plan.before);
    let mut saved: HashMap<_, _> = plan.after.into_iter().map(|(key, t)| (key, omsi_map::Terrain::parse(&t.to_bytes()).unwrap())).collect();
    assert_ne!(saved[&(0, 0)], t);
    editor.undo.pop().unwrap().restore_terrain(&mut saved);
    assert_eq!(saved[&(0, 0)], t);
    assert_eq!(omsi_map::Terrain::parse(&saved[&(0, 0)].to_bytes()).unwrap(), t);
}

fn replacement_fixture() -> Edits {
    let mut source = between("road.sli", 1, (0, 0), DVec3::new(50.0, 50.0, 0.0), DVec3::new(50.0, 60.0, 0.0), 0.0);
    let mut target = between("road.sli", 2, (0, 0), DVec3::new(50.0, 60.3, 0.0), DVec3::new(50.0, 80.0, 0.0), 0.0);
    source.prev_id = 8; source.next_id = 9;
    target.prev_id = 10; target.next_id = 11;
    let mut old_source_neighbor = source.clone(); old_source_neighbor.id = 9;
    old_source_neighbor.prev_id = 1; old_source_neighbor.next_id = 12;
    let mut old_target_neighbor = target.clone(); old_target_neighbor.id = 10;
    old_target_neighbor.prev_id = 13; old_target_neighbor.next_id = 2;
    Edits { originals: HashMap::from([
        (((0, 0), 1), source), (((0, 0), 2), target),
        (((0, 0), 9), old_source_neighbor), (((0, 0), 10), old_target_neighbor),
    ]), ..Default::default() }
}

#[test]
fn replacement_previews_both_reciprocal_links_without_mutating_the_map() {
    let edits = replacement_fixture();
    assert!(connection_plan(&edits, &[(0, 0)], None, ((0, 0), 1), ((0, 0), 2), false, false).unwrap_err().contains("occupied by 9"));
    let preview = connection_plan(&edits, &[(0, 0)], None, ((0, 0), 1), ((0, 0), 2), true, false).unwrap();
    assert_eq!((preview.source.prev_id, preview.source.next_id), (8, 2));
    assert_eq!((preview.target.prev_id, preview.target.next_id), (1, 11));
    assert_eq!(preview.neighbors.len(), 2);
    assert_eq!((preview.neighbors[0].1.prev_id, preview.neighbors[0].1.next_id), (0, 12));
    assert_eq!((preview.neighbors[1].1.prev_id, preview.neighbors[1].1.next_id), (13, 0));
    assert_eq!(edits.current(((0, 0), 1)).unwrap().next_id, 9);
    assert_eq!(edits.current(((0, 0), 9)).unwrap().prev_id, 1);
    assert!(edits.changed.is_empty());
    let mut changed = HashMap::from([(1, preview.source), (2, preview.target)]);
    for (_, neighbor) in preview.neighbors { changed.insert(neighbor.id, neighbor); }
    let original = format!("[version]\n14\n{}", [1, 2, 9, 10].map(|id| record(&edits.current(((0, 0), id)).unwrap(), "0", "\n")).join("\n"));
    let saved = rewrite(&original, &changed, &[]).unwrap().0;
    let tile = parsed(&saved);
    assert_eq!(tile.splines.iter().map(|s| (s.id, s.prev_id, s.next_id)).collect::<Vec<_>>(),
        vec![(1, 8, 2), (2, 1, 11), (9, 0, 12), (10, 13, 0)]);
}

#[test]
fn replacement_distinguishes_missing_neighbors_from_unloaded_or_unread_tiles() {
    let mut edits = replacement_fixture();
    edits.originals.get_mut(&((0, 0), 1)).unwrap().next_id = 99;
    edits.originals.get_mut(&((0, 0), 2)).unwrap().prev_id = 0;
    let mut index = crate::tiles::MapIndex::default();
    index.splines.insert(99, crate::tiles::IndexedSpline { length: 10.0, map_chain_offset: None, prev: 1, next: 0 });
    assert!(connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap_err().contains("99 for editing first"));
    index.splines.remove(&99);
    let preview = connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap();
    assert_eq!(preview.source.next_id, 2);
    assert!(preview.neighbors.is_empty());
    index.tiles_failed = 1;
    assert!(connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap_err().contains("not fully"));
    let mut neighbor = edits.current(((0, 0), 9)).unwrap(); neighbor.id = 99;
    edits.originals.insert(((1, 0), 99), neighbor);
    assert!(connection_plan(&edits, &[(0, 0)], Some(&index), ((0, 0), 1), ((0, 0), 2), true, false).unwrap_err().contains("99 first"));
    assert_eq!(edits.current(((0, 0), 1)).unwrap().next_id, 99);
}

#[test]
fn replacement_merges_two_detachments_from_the_same_old_neighbor() {
    let mut edits = replacement_fixture();
    edits.originals.get_mut(&((0, 0), 2)).unwrap().prev_id = 9;
    edits.originals.get_mut(&((0, 0), 9)).unwrap().next_id = 2;
    let preview = connection_plan(&edits, &[(0, 0)], None, ((0, 0), 1), ((0, 0), 2), true, false).unwrap();
    assert_eq!(preview.neighbors.len(), 1);
    assert_eq!((preview.neighbors[0].1.prev_id, preview.neighbors[0].1.next_id), (0, 0));
    let old = edits.current(((0, 0), 9)).unwrap();
    assert_eq!((old.prev_id, old.next_id), (1, 2));
}
