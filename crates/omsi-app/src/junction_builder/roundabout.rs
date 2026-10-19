pub use omsi_geometry::roundabout::{Roundabout,build};
#[cfg(test)]
use {super::*,omsi_geometry::roundabout::{on_road,edge_strips,entrance_dashes,delta,mouth_angle}};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lane_controls_roundtrip_undo_and_old_project_defaults() {
        let mut w=Window::new_roundabout(false);let before=w.project.clone();
        w.set(Field::RingLanes,3.0);assert_eq!(w.project.roundabout.as_ref().unwrap().lanes,3);assert_eq!(w.value(Field::RingWidth),18.0);
        w.set(Field::ApproachLanes,2.0);assert_eq!(w.value(Field::Width),14.0);assert_eq!(w.project.roundabout.as_ref().unwrap().approach_lanes,[2,1,1,1]);
        let value=serde_json::to_value(&w.project).unwrap();let restored:Project=serde_json::from_value(value).unwrap();assert_eq!(restored,w.project);
        w.command(Command::Undo);w.command(Command::Undo);assert_eq!(w.project,before);
        let mut json=serde_json::to_value(&before).unwrap();let obj=json["roundabout"].as_object_mut().unwrap();obj.remove("lanes");obj.remove("approach_lanes");obj.remove("lane_lines");
        let old:Project=serde_json::from_value(json).unwrap();assert_eq!(old,before);
        w.edit(Field::RingLanes);w.input.as_mut().unwrap().text="2.5".into();w.input.as_mut().unwrap().replace=false;assert!(!w.commit());assert_eq!(w.project,before);
    }
    #[test]
    fn workflow_switch_commits_valid_input_and_preserves_invalid_input() {
        let mut w=Window::new_roundabout(false);
        w.edit(Field::IslandRadius);w.input.as_mut().unwrap().text="14".into();w.input.as_mut().unwrap().replace=false;
        w.command(Command::RoundaboutStep(1));assert_eq!(w.roundabout_step,1);assert_eq!(w.value(Field::IslandRadius),14.0);
        w.edit(Field::Width);w.input.as_mut().unwrap().text="invalid".into();w.input.as_mut().unwrap().replace=false;
        w.command(Command::RoundaboutStep(2));assert_eq!(w.roundabout_step,1);assert!(w.input.is_some());
        w.input=None;let before=serde_json::to_string(&w.project).unwrap();
        for step in [2,3,0,1] {w.command(Command::RoundaboutStep(step));assert_eq!(w.roundabout_step,step);}
        assert_eq!(before,serde_json::to_string(&w.project).unwrap());
    }
    #[test]
    #[ignore = "requires graphics adapter; writes roundabout UI screenshots"]
    fn render_splitter_ui() {
        let instance=crate::graphics_instance();let mut renderer=pollster::block_on(omsi_render::Renderer::new_with(&instance,None,Some(wgpu::TextureFormat::Rgba8UnormSrgb),omsi_render::RenderOptions{msaa:1,ssao:false,fxaa:false,shadow_size:1024,..Default::default()})).unwrap();
        let camera=omsi_render::Camera{position:DVec3::new(0.0,0.0,10.0),yaw:0.0,pitch:0.0,roll:0.0,fov_deg:60.0,near:0.1,far:1000.0};
        for lang in ["DEU","ENG"] {crate::ui_language(lang);for (width,height) in [(1280,800),(800,600)] {
            let mut w=Window::new_roundabout(false);w.set(Field::RingLanes,2.0);w.set(Field::ApproachLanes,2.0);
            let mut scene=renderer.new_scene();let mut ui=crate::ui::Ui::new().unwrap();
            ui.draw_junction_window(&renderer,&mut scene,Some(&mut w),Path::new("/tmp"),1.0,[0.0,0.0,width as f32,height as f32],(-1.0,-1.0));
            for step in [1,3,0,2] {
                let r=w.rects.iter().find(|(_,c)|matches!(c,Command::RoundaboutStep(n) if *n==step)).unwrap().0;
                for cursor in [(r[0]+1.0,r[1]+1.0),((r[0]+r[2])*0.5,(r[1]+r[3])*0.5),(r[2]-1.0,r[3]-1.0)] {
                    assert!(!ui.editor_window_drag_press(cursor),"island button must not start window dragging");
                    assert!(matches!(w.hit(cursor),Some(Command::RoundaboutStep(n)) if n==step));
                }
                w.command(w.hit(((r[0]+r[2])*0.5,(r[1]+r[3])*0.5)).unwrap());assert_eq!(w.roundabout_step,step);assert_eq!(w.splitter_settings,step==2);
                scene.overlays.clear();ui.draw_junction_window(&renderer,&mut scene,Some(&mut w),Path::new("/tmp"),1.0,[0.0,0.0,width as f32,height as f32],(-1.0,-1.0));
                let rgba=renderer.render_to_image(&mut scene,width,height,&camera,&omsi_render::Lighting{shadows:false,..Default::default()}).unwrap();
                image::save_buffer(format!("/tmp/roundabout-step-{step}-{lang}-{width}.png"),&rgba,width,height,image::ColorType::Rgba8).unwrap();
            }
            let r=w.rects.iter().find(|(_,c)|matches!(c,Command::Splitter)).unwrap().0;let cursor=((r[0]+r[2])*0.5,(r[1]+r[3])*0.5);
            assert!(!ui.editor_window_drag_press(cursor));w.command(w.hit(cursor).unwrap());assert!(w.project.roundabout.as_ref().unwrap().splitters[0].enabled);
            scene.overlays.clear();ui.draw_junction_window(&renderer,&mut scene,Some(&mut w),Path::new("/tmp"),1.0,[0.0,0.0,width as f32,height as f32],(-1.0,-1.0));
            let r=w.rects.iter().find(|(_,c)|matches!(c,Command::SplitterTemplate)).unwrap().0;let cursor=((r[0]+r[2])*0.5,(r[1]+r[3])*0.5);
            assert!(!ui.editor_window_drag_press(cursor));w.command(w.hit(cursor).unwrap());assert_eq!(w.project.roundabout.as_ref().unwrap().splitters[0].shape,omsi_geometry::roundabout_splitters::Shape::Wedge);
            scene.overlays.clear();ui.draw_junction_window(&renderer,&mut scene,Some(&mut w),Path::new("/tmp"),1.0,[0.0,0.0,width as f32,height as f32],(-1.0,-1.0));
            assert!(w.error.is_none(),"{:?}",w.error);assert!(w.rects.iter().any(|(_,c)|matches!(c,Command::Splitter)));
            for (r,_) in &w.rects {assert!(r[0]>=0.0&&r[1]>=0.0&&r[2]<=width as f32&&r[3]<=height as f32);}
            let rgba=renderer.render_to_image(&mut scene,width,height,&camera,&omsi_render::Lighting{shadows:false,..Default::default()}).unwrap();
            image::save_buffer(format!("/tmp/roundabout-ui-{lang}-{width}.png"),&rgba,width,height,image::ColorType::Rgba8).unwrap();
        }}
    }
    #[test]
    fn splitter_controls_are_per_arm_and_undoable() {
        let mut w=Window::new_roundabout(false);let initial=w.project.clone();
        w.command(Command::RoundaboutStep(2));w.command(Command::Splitter);w.command(Command::SplitterShape);w.set(Field::SplitterLength,5.0);
        assert!(w.splitter_settings);assert!(w.project.roundabout.as_ref().unwrap().splitters[0].enabled);
        w.command(Command::Arm(1));assert!(!w.project.roundabout.as_ref().unwrap().splitters[1].enabled);assert_eq!(w.value(Field::SplitterLength),6.0);
        for _ in 0..3 {w.command(Command::Undo);}assert_eq!(w.project,initial);
        for _ in 0..3 {w.command(Command::Redo);}assert_eq!(w.project.roundabout.as_ref().unwrap().splitters[0].length,5.0);build(&w.project).unwrap();
    }
    #[test]
    fn reference_island_preset_preserves_other_arms_and_undo() {
        let mut w=Window::new_roundabout(false);let original=w.project.clone();w.arm=2;w.command(Command::SplitterTemplate);
        let r=w.project.roundabout.as_ref().unwrap();assert!(r.splitters[2].enabled);assert!(!r.splitters[0].enabled);
        assert_eq!((r.splitters[2].gap,r.splitters[2].length),(1.0,9.0));build(&w.project).unwrap();
        let expected=w.project.clone();w.command(Command::Undo);assert_eq!(w.project,original);w.command(Command::Redo);assert_eq!(w.project,expected);
        w.command(Command::SplitterShape);assert_eq!(w.project.roundabout.as_ref().unwrap().splitters[2].shape,omsi_geometry::roundabout_splitters::Shape::Rounded);
    }
    #[test]
    #[ignore = "Writes splitter island visual QA previews"]
    fn render_splitter_preview() {
        for shape in [omsi_geometry::roundabout_splitters::Shape::Rounded,omsi_geometry::roundabout_splitters::Shape::Teardrop,omsi_geometry::roundabout_splitters::Shape::Wedge] {
            let mut w=Window::new_roundabout(false);for s in &mut w.project.roundabout.as_mut().unwrap().splitters {s.enabled=true;s.shape=shape;if shape==omsi_geometry::roundabout_splitters::Shape::Wedge{s.gap=1.0;s.length=9.0;}}
            w.project.roundabout.as_mut().unwrap().outer_line=true;let mut b=build(&w.project).unwrap();path_overlay(&mut b.mesh,&b.paths);
            let img=crate::asset_catalog::junction_preview(std::path::Path::new("/tmp"),&b.mesh,0,None).unwrap();
            image::RgbaImage::from_raw(img.width,img.height,img.rgba).unwrap().save(format!("/tmp/roundabout-splitter-{shape:?}.png")).unwrap();
        }
    }
    #[test]
    #[ignore = "Writes a local visual QA preview when requested"]
    fn render_edge_line_preview() {
        let mut w=Window::new_roundabout(false);w.command(Command::Shape(false));
        for a in &mut w.project.arms {a.width=10.0;a.length=35.0;}
        let r=w.project.roundabout.as_mut().unwrap();r.road_width=12.0;r.outer_line=true;r.dashed_entries=true;r.line_insets=[[0.8,0.8];4];
        let b=super::build(&w.project).unwrap();
        let img=crate::asset_catalog::junction_preview(std::path::Path::new("/tmp"),&b.mesh,0,None).unwrap();
        image::RgbaImage::from_raw(img.width,img.height,img.rgba).unwrap().save("/tmp/editor-bautest8-roundabout.png").unwrap();
    }

    #[test]
    fn edge_lines_follow_boundary_match_arm_offsets_and_leave_ports_open() {
        for count in [3,4] {
            let mut w=Window::new_roundabout(false);w.command(Command::Shape(count==4));
            for arm in &mut w.project.arms {arm.angle=(arm.angle+17.0)%360.0;arm.width=10.0;arm.length=40.0;}
            let r=w.project.roundabout.as_mut().unwrap();r.road_width=8.0;r.outer_line=true;r.line_width=0.15;
            r.line_insets=[[0.6,0.9];4];
            let b=super::build(&w.project).unwrap();let strips=edge_strips(&w.project,&b.outline);
            assert!(!strips.is_empty());
            for q in &strips {for v in q {assert!(on_road(&w.project,*v),"{v:?}");}}
            for a in w.project.arms.iter().filter(|a|a.enabled) {
                let d=direction(a.angle);let right=right(d);
                let ends=strips.iter().flat_map(|q|[(q[0]+q[1])/2.0,(q[2]+q[3])/2.0])
                    .filter(|v|(v.truncate().dot(d)-a.length).abs()<1e-5).collect::<Vec<_>>();
                assert_eq!(ends.len(),2);
                assert!(ends.iter().any(|v|(v.truncate().dot(right)-(a.width/2.0-0.6)).abs()<1e-5));
                assert!(ends.iter().any(|v|(v.truncate().dot(right)+(a.width/2.0-0.9)).abs()<1e-5));
            }
            let json=serde_json::to_vec(&w.project).unwrap();let restored:Project=serde_json::from_slice(&json).unwrap();assert_eq!(w.project,restored);
            w.command(Command::OuterLine);assert!(!w.project.roundabout.as_ref().unwrap().outer_line);
            w.command(Command::Undo);assert!(w.project.roundabout.as_ref().unwrap().outer_line);
        }
    }
    #[test]
    fn dashed_entries_are_separate_and_only_cover_openings() {
        let mut w=Window::new_roundabout(false);w.command(Command::Shape(false));
        w.command(Command::DashedEntries);let b=super::build(&w.project).unwrap();assert!(!b.mesh.triangles.is_empty());
        let p=&w.project;let r=p.roundabout.as_ref().unwrap();let dashes=entrance_dashes(p);assert!(!dashes.is_empty());
        for q in &dashes {for v in q {
            assert!(on_road(p,*v));let theta=v.x.atan2(v.y);
            assert!(p.arms.iter().filter(|a|a.enabled).any(|a|delta(theta,a.angle.to_radians()).abs()<=mouth_angle(r.island_radius+r.road_width,a.width/2.0)+1e-6));
        }}
        let mut json=serde_json::to_value(p).unwrap();for name in ["outer_line","dashed_entries","line_width","line_insets","splitters"] {json["roundabout"].as_object_mut().unwrap().remove(name);}
        let old:Project=serde_json::from_value(json).unwrap();assert!(old.roundabout.as_ref().unwrap().splitters.iter().all(|s|!s.enabled));assert!(!old.roundabout.as_ref().unwrap().outer_line);super::build(&old).unwrap();
    }

    #[test]
    fn both_traffic_sides_all_exits_reachable_and_no_island_crossing() {
        for left in [false,true] {for count in [3,4] {
            let mut w=Window::new_roundabout(left);w.command(Command::Shape(count==4));
            let p=&w.project;let b=super::build(p).unwrap();let r=p.roundabout.as_ref().unwrap();
            assert_eq!(b.paths.len(),count*4);
            // Ring connects cyclically, including every entry and exit node exactly.
            let ring=&b.paths[..count*2];
            for (i,(pts,_,_)) in ring.iter().enumerate() {
                assert_eq!(pts.last(),ring[(i+1)%ring.len()].0.first());
                for pair in pts.windows(2) {
                    assert!(pair[0].truncate().perp_dot(pair[1].truncate())*if left{-1.0}else{1.0}>0.0);
                    assert!(((pair[0]+pair[1])*0.5).truncate().length()>r.island_radius);
                }
            }
            for i in 0..count {
                let incoming=&b.paths[count*2+i].0;let outgoing=&b.paths[count*3+i].0;
                let next=ring.iter().find(|v|v.0.first()==incoming.last()).unwrap();
                let in_heading=(incoming[incoming.len()-1]-incoming[incoming.len()-2]).normalize();
                let ring_heading=(next.0[1]-next.0[0]).normalize();
                assert!(in_heading.dot(ring_heading)>0.995, "entry must merge tangentially");
                assert!(ring.iter().any(|v|v.0.first()==outgoing.first()));
            }
            assert!(b.paths.iter().all(|v|v.0.iter().all(|&q|on_road(p,q))));
            let restored=omsi_o3d::parse_o3d(&o3d(&b.mesh).unwrap()).unwrap();
            assert_eq!(restored.triangles,b.mesh.triangles);
            assert!(restored.vertices.iter().all(|v|v.position.is_finite()&&v.uv.is_finite()));
            let object=omsi_scenery::SceneryObject::parse(&omsi_cfg::CfgFile::from_str("ring.sco",&sco(p,&b)));
            assert!(object.paths.len()>100);assert!(object.paths.iter().all(|p|p.direction==0));
        }}
    }
    #[test]
    fn exported_roundabout_reopens_and_project_load_detaches_old_target() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("roundabout-export-{stamp}"));
        let original=dir.join("original");let content=dir.join("content");std::fs::create_dir_all(&original).unwrap();
        let mut w=Window::new_roundabout(false);w.command(Command::SplitterTemplate);w.project.road_texture="Texture/asphalt.png".into();
        std::fs::create_dir_all(original.join("Texture")).unwrap();
        image::RgbaImage::from_pixel(4,4,image::Rgba([50,60,70,255])).save(original.join("Texture/asphalt.png")).unwrap();
        let sco_path=export(&w.project,&original,&content,&original).unwrap();
        assert!(sco_path.is_file());assert!(sco_path.parent().unwrap().join("texture/asphalt.png").is_file());
        let mesh=omsi_o3d::load_mesh(&sco_path.parent().unwrap().join("model/road.o3d")).unwrap();assert!(!mesh.triangles.is_empty());
        let rules=roundabout_rules(&sco_path).unwrap().unwrap();assert!(rules.contains("priority\n192"));
        let project=w.project.clone();w.target=Some(123);w.placed_project=Some(project.clone());w.pending=Some((((0,0),7),0));
        w.load(&sco_path.parent().unwrap().join("junction.junction.json")).unwrap();
        assert_eq!(w.project,project);assert!(w.target.is_none()&&w.placed_project.is_none()&&w.pending.is_none());
        assert!(!original.join("Sceneryobjects").exists());
        let second=export(&w.project,&original,&content,&original).unwrap();assert_ne!(second,sco_path);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn old_projects_new_roundtrip_and_bad_geometry() {
        let old:Project=serde_json::from_str(&serde_json::to_string(&Project::default()).unwrap()).unwrap();
        assert!(old.roundabout.is_none());assert!(super::super::build(&old).is_ok());
        let mut w=Window::new_roundabout(false);let initial=w.project.clone();
        w.set(Field::IslandRadius,14.0);w.command(Command::Undo);assert_eq!(w.project,initial);
        w.command(Command::Redo);assert_eq!(w.value(Field::IslandRadius),14.0);
        let restored:Project=serde_json::from_slice(&serde_json::to_vec(&w.project).unwrap()).unwrap();assert_eq!(restored,w.project);
        for bad in [f64::NAN,0.0,1000.0] {
            let mut p=initial.clone();p.roundabout.as_mut().unwrap().island_radius=bad;assert!(super::super::build(&p).is_err());
        }
        let mut p=initial.clone();p.arms[0].length=20.0;assert!(super::super::build(&p).is_err());
        p=initial.clone();p.arms[0].angle=p.arms[1].angle;assert!(super::super::build(&p).is_err());
        p=initial.clone();p.arms[0].bend=5.0;assert!(super::super::build(&p).is_err());
        for (island,width) in [(12.0,6.0),(18.0,8.0),(30.0,10.0)] {
            let mut p=initial.clone();let r=p.roundabout.as_mut().unwrap();r.island_radius=island;r.road_width=width;
            for a in &mut p.arms {a.angle=(a.angle+15.0)%360.0;a.length=island+width+15.0;a.width=6.0;}
            let b=super::super::build(&p).unwrap();assert!(b.paths.iter().all(|v|v.0.iter().all(|&q|on_road(&p,q))));
        }
        // Legacy builders reject format 2 instead of silently exporting a T/X junction.
        assert_eq!(initial.format,2);
        for i in 0..4 {let (at,heading,_)=port(&initial,i).unwrap();assert!((at.length()-35.0).abs()<1e-8);assert_eq!(heading,initial.arms[i].angle);}
    }
}
