//! Exercise the exported SCO through the real loader and traffic linker.
use super::*;
use omsi_sim::traffic::{Network, LaneBuilder, LaneKind, AiState, LINK_TOLERANCE};

#[test]
fn exported_roundabout_keeps_internal_joints_exact_and_external_roads_tolerant() {
    for left in [false,true] { for count in [2,4] {
        let mut w=Window::new_roundabout(left);
        w.set(Field::RingLanes,count as f64);w.set(Field::ApproachLanes,count as f64);
        let b=build(&w.project).unwrap();let text=sco(&w.project,&b);
        let object=omsi_scenery::SceneryObject::parse(&omsi_cfg::CfgFile::from_str("test.sco",&text));
        assert!(object.editor_exact_path_links);
        let lanes=omsi_scene::lanes::object_lanes(&object,DVec3::new(-4500.0,-2800.0,12.0),[129.0,0.0,0.0],None,(0,0),99,&[]);
        assert!(lanes.iter().all(|l|l.editor_exact_path_links));
        let mut net=Network{lanes:lanes.clone(),left_hand:left,..Default::default()};net.link(LINK_TOLERANCE);
        let ports:Vec<_>=net.lanes.iter().enumerate().filter(|(_,l)|l.next.is_empty()).map(|(i,_)|i).collect();
        assert_eq!(ports.len(),w.project.arms.iter().filter(|a|a.enabled).enumerate().map(|(i,_)|w.project.roundabout.as_ref().unwrap().approach_lanes[i] as usize).sum::<usize>());
        for l in &net.lanes {for &j in &l.next {assert!(l.end().distance(net.lanes[j].start())<0.002);}}
        // Demonstrate the regression: normal loose linking splices the short samples
        // into other curves and back into previous pieces of the same curve.
        let mut old=Network{lanes:lanes.clone(),left_hand:left,..Default::default()};for l in &mut old.lanes {l.editor_exact_path_links=false;}old.link(LINK_TOLERANCE);
        let false_links=old.lanes.iter().map(|l|l.next.iter().filter(|&&j|l.end().distance(old.lanes[j].start())>0.002).count()).sum::<usize>();
        assert!(false_links>100,"fixture must reproduce the old false connections");
        // An ordinary road a little away from an outlet must still connect. Make
        // every outlet long enough that the dead-end filter permits choosing it.
        let count=net.lanes.len();
        for &i in &ports {let l=&net.lanes[i];let d=(l.end()-l.start()).normalize();let start=l.end()+d*0.3;
            net.lanes.push(LaneBuilder::polyline(vec![start,start+d*700.0],LaneKind::Street,l.width));}
        net.link(LINK_TOLERANCE);
        for &i in &ports {assert!(net.lanes[i].next.iter().any(|&j|j>=count));}
        // Incremental map loading uses the same precision rule as a full reload.
        let split=lanes.len()/2;let mut grown=Network{lanes:lanes[..split].to_vec(),left_hand:left,..Default::default()};grown.link(LINK_TOLERANCE);
        grown.extend(net.lanes[split..].to_vec(),LINK_TOLERANCE);
        for i in 0..net.lanes.len(){let mut a=net.lanes[i].next.clone();let mut b=grown.lanes[i].next.clone();a.sort_unstable();b.sort_unstable();assert_eq!(a,b);}
        // Random traffic can actually take an outlet, not merely reach it in BFS.
        for seed in 1..=16 {let mut driver=AiState::new(0,0.0,seed);let mut distance=0.0;
            while driver.lane<count && distance<4000.0 {driver.plan_next(&net);let next=driver.planned_next.expect("continuous ring route");distance+=net.lanes[driver.lane].length();driver.lane=next;driver.s=0.0;driver.planned_next=None;driver.ahead.clear();}
            assert!(driver.lane>=count,"driver {seed} failed to leave the ring");
        }
    }}
}
