use super::*;
use crate::junction_connections::Pose;
#[test]
fn tilted_builder_connections_follow_pose_keep_far_edges_and_roundtrip() {
    for roundabout in [false,true] {for reverse in [false,true] {for mirrored in [false,true] {
        let mut project=if roundabout{crate::junction_builder::Window::new_roundabout(false).project}else{crate::junction_builder::Window::new(false).project};
        // Both T/three-arm and X/four-arm configurations.
        for four in [false,true] {project.arms[3].enabled=four;
            for arm in 0..4 {if !project.arms[arm].enabled{continue;}
                let (local,angle,def)=crate::junction_builder::port(&project,arm).unwrap();
                let old=Pose{at:DVec3::new(150.0,150.0,2.0),heading:15.0,tilt:[0.0;2]};
                let old_port=old.port(local,angle).unwrap();
                let far=old_port.point+SplineCurve::dir(old_port.outward).extend(0.0)*70.0;
                let (a,b)=if reverse{(far,old_port.point)}else{(old_port.point,far)};
                let tile=(0,0);let mut road=between("test.sli",51,tile,a,b,0.0);road.mirror=mirrored;
                let end=if reverse{End::Finish}else{End::Start};let distant=if reverse{End::Start}else{End::Finish};
                end.set_link(&mut road,123);distant.set_link(&mut road,999);
                let fixed=edge_points(tile,&road,&def,distant);
                for pose in [Pose{at:old.at+DVec3::new(0.4,0.6,0.3),heading:20.0,tilt:[3.0,-2.0]},old] {
                    let port=pose.port(local,angle).unwrap();
                    let fit=junction_fit_port(tile,&road,&def,123,port,&def).unwrap_or_else(|e|panic!("round={roundabout} four={four} arm={arm} reverse={reverse} mirror={mirrored}: {e}"));
                    let mut expected=physical_edges(&def,false).map(|(x,z)|port.surface(x,z));
                    if end==End::Start{expected.swap(0,1);}
                    for (got,want) in edge_points(tile,&fit,&def,end).iter().zip(expected){assert!(got.distance(want)<0.001);}
                    for (got,want) in edge_points(tile,&fit,&def,distant).iter().zip(fixed){assert!(got.distance(want)<0.005);}
                    assert!((curve(tile,&fit).slope_at(if reverse{fit.length}else{0.0})*100.0-port.grade*if reverse{-1.0}else{1.0}).abs()<1e-7);
                    assert_eq!(end.link(&fit),123);assert_eq!(distant.link(&fit),999);assert_eq!(fit.mirror,mirrored);
                    let saved=record(&fit,"0","\n");
                    let parsed=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_str("test.map",&saved));
                    let restored=&parsed.splines[0];
                    for(got,want)in edge_points(tile,restored,&def,end).iter().zip(expected){assert!(got.distance(want)<0.001);}
                    let again=junction_fit_port(tile,&fit,&def,123,port,&def).unwrap();
                    for n in 0..=10 {assert!(curve(tile,&fit).point_at(fit.length*n as f64/10.0).distance(curve(tile,&again).point_at(again.length*n as f64/10.0))<1e-6);}
                    road=fit;
                }
            }
        }
    }}}
}
#[test]
fn junction_connection_rejects_occupied_or_impossible_fit_without_mutation() {
    let project=crate::junction_builder::Window::new_roundabout(false).project;
    let(local,angle,def)=crate::junction_builder::port(&project,0).unwrap();
    let pose=Pose{at:DVec3::new(100.0,100.0,0.0),heading:0.0,tilt:[0.0;2]};let port=pose.port(local,angle).unwrap();
    let far=port.point+SplineCurve::dir(port.outward).extend(0.0)*60.0;
    let mut road=between("test.sli",50,(0,0),port.point,far,0.0);road.prev_id=55;
    let before=road.clone();assert!(junction_fit_port((0,0),&road,&def,123,port,&def).is_err());assert_eq!(road,before);
    road.prev_id=123;let before=road.clone();
    let invalid=Pose{at:pose.at+DVec3::new(200.0,0.0,0.0),..pose}.port(local,angle).unwrap();
    assert!(junction_fit_port((0,0),&road,&def,123,invalid,&def).is_err());assert_eq!(road,before);
    assert!(Pose{tilt:[89.0,0.0],..pose}.port(local,angle).is_err());
    // A linked endpoint is selected by ID even when the other endpoint becomes closer.
    assert_eq!(junction_end(&road,&curve((0,0),&road),123,far),End::Start);
}
