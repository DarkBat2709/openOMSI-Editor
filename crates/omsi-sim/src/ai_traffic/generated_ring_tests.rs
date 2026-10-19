use super::*;
use super::scenario_tests::{Fixture,traffic,add_car};
use crate::traffic::{LaneBuilder,LaneKey};
#[test]
fn generated_two_lane_ring_routes_cars_out_without_body_overlap(){
    for islands in [false,true] {for seed in 1..=8 {
    let mut p=omsi_geometry::junction::Project::default();
    p.roundabout=Some(omsi_geometry::roundabout::Roundabout{lanes:2,approach_lanes:[2;4],road_width:12.0,..Default::default()});
    for (i,a) in p.arms.iter_mut().enumerate(){a.enabled=i<3;a.width=14.0;a.length=54.0;}
    if islands {for s in &mut p.roundabout.as_mut().unwrap().splitters[..3]{s.enabled=true;s.shape=omsi_geometry::roundabout_splitters::Shape::Wedge;s.length=10.0;s.width=2.5;s.gap=1.0;}}
    let built=omsi_geometry::roundabout::build(&p).unwrap();let mut lanes=Vec::new();let mut entries=Vec::new();let mut exits=Vec::new();
    for (i,(points,width,turn)) in built.paths.iter().enumerate(){
        let mut l=LaneBuilder::polyline(points.clone(),LaneKind::Street,*width as f32);l.key=Some(LaneKey{tile:(0,0),id:9,path:i as u16});l.source=2;l.editor_exact_path_links=true;l.editor_grouped_path=true;l.speed_limit_kmh=20.0;l.turn=if *turn==3{2}else{0};
        if l.start().truncate().length()>50.0 {entries.push(i);l.priority=64.0;}else{l.priority=192.0;}
        if l.end().truncate().length()>50.0 {exits.push(i);}
        lanes.push(l);
    }
    let count=lanes.len();for i in exits {let l=&lanes[i];let d=(l.end()-l.points[l.points.len()-2]).normalize();lanes.push(LaneBuilder::polyline(vec![l.end(),l.end()+d*700.0],LaneKind::Street,l.width));}
    let mut net=Network{lanes,..Default::default()};net.link(1.5);let f=Fixture::new();let mut t=traffic(&f,net);
    for (k,i) in entries.into_iter().enumerate(){add_car(&mut t,&f,i,0.0,seed+k as u64*0x1000,Some(4.0));}
    let mid=t.net.lanes[0].length()/2.0;add_car(&mut t,&f,0,mid,seed+0x100000,Some(4.0));
    for step in 0..1600 {t.tick(0.05,None);let feet=t.footprints();for (i,a) in feet.iter().enumerate(){for b in feet.iter().skip(i+1){assert!(!a.overlaps(b,-0.15),"overlap at step {step}: {} / {}",a.car,b.car);}}}
    assert!(t.cars.iter().all(|c|c.state.lane>=count),"cars failed to leave the generated ring");
    }}
}
