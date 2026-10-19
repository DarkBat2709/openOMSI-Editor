//! A transient, non-pickable spline ghost. No map edits, IDs or files are created.
use glam::{DVec2,DVec3,Mat4};
use omsi_render::{Renderer,Scene,AlphaMode};
use std::hash::{Hash,Hasher};
pub fn draw(world:&crate::World,r:&Renderer,scene:&mut Scene,proposal:Option<omsi_map::MapSpline>) {
    let proposal=proposal.and_then(|s|world.spline_type(&s.file).map(|ty|(s,ty)));
    let Some((s,ty))=proposal else {if let Some((_,_,_,i))=scene.editor_spline_preview {r.set_params(scene,i,&[],false,&[]);}return;};
    let mut hash=std::collections::hash_map::DefaultHasher::new();s.file.hash(&mut hash);
    for n in s.pos.into_iter().chain([s.heading,s.length,s.radius,s.grad_start,s.grad_end]){n.to_bits().hash(&mut hash);}let hash=hash.finish();
    if let Some((old,_,_,i))=scene.editor_spline_preview {if old==hash {r.set_params(scene,i,&[],true,&[]);return;}}
    let origin=DVec3::from(s.pos);let curve=omsi_geometry::SplineCurve::from_map(&s,DVec2::ZERO).with_sli(&ty.def);
    let mut data=omsi_geometry::build_spline_mesh(&ty.def,&curve,s.mirror,origin);
    for range in &mut data.ranges {range.2=0;} data.one_sided=false;
    let old=scene.editor_spline_preview;
    let mesh=r.add_mesh(scene,&data);
    let mesh=if let Some((_,m,_,_))=old {r.free_mesh(scene,m);r.recycle_mesh(scene,mesh,m)}else{mesh};
    let mat=old.map(|(_,_,m,_)|m).unwrap_or_else(||r.add_material(scene,None,AlphaMode::Blend,[0.1,0.75,1.0,0.55],true));
    let i=r.add_instance(scene,mesh,origin+DVec3::Z*0.03,Mat4::IDENTITY,vec![mat]);
    let i=if let Some((_,_,_,prior))=old {r.remove_instance(scene,prior);r.recycle_instance(scene,i,prior)}else{i};
    scene.editor_spline_preview=Some((hash,mesh,mat,i));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires graphics adapter"]
    fn ghost_updates_hides_and_reuses_resources_without_map_edits() {
        let root=std::env::temp_dir().join(format!("spline-ghost-{}",std::process::id()));let map=root.join("maps/Test");
        std::fs::create_dir_all(&map).unwrap();std::fs::create_dir_all(root.join("Splines")).unwrap();
        std::fs::write(root.join("Splines/road.sli"),"[profile]\n0\n[profilepnt]\n-3\n0\n0\n1\n[profilepnt]\n3\n0\n1\n1\n").unwrap();
        std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n").unwrap();std::fs::write(map.join("tile_0_0.map"),"[version]\n14\n").unwrap();
        let world=crate::World::open(&root,&map.join("global.cfg"),20000101).unwrap();
        let instance=wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let mut r=pollster::block_on(Renderer::new_with(&instance,None,Some(wgpu::TextureFormat::Rgba8UnormSrgb),omsi_render::RenderOptions{msaa:1,ssao:false,fxaa:false,shadow_size:1024,..Default::default()})).unwrap();
        let mut scene=r.new_scene();
        let mut s=omsi_map::MapSpline{file:"Splines/road.sli".into(),pos:[0.0,0.0,1.0],length:20.0,radius:20.0/std::f64::consts::FRAC_PI_2,..Default::default()};
        draw(&world,&r,&mut scene,Some(s.clone()));let first=scene.editor_spline_preview.unwrap();let old_radius=scene.meshes[first.1].bounds_radius;
        s.length=40.0;s.radius*=2.0;draw(&world,&r,&mut scene,Some(s.clone()));let second=scene.editor_spline_preview.unwrap();
        assert_ne!(first.0,second.0);assert_eq!((first.1,first.2,first.3),(second.1,second.2,second.3));assert!(scene.meshes[second.1].bounds_radius>old_radius);
        assert_eq!((scene.meshes.len(),scene.materials.len(),scene.instances.len()),(1,1,1));
        assert!(world.spline_edits.lock().added.is_empty());assert!(!world.spline_edits.lock().dirty);
        let camera=omsi_render::Camera{position:DVec3::new(13.0,13.0,60.0),yaw:0.0,pitch:-90.0,roll:0.0,fov_deg:60.0,near:0.1,far:1000.0};
        let pixels=r.render_to_image(&mut scene,800,600,&camera,&omsi_render::Lighting{shadows:false,..Default::default()}).unwrap();
        image::save_buffer("/tmp/spline-live-preview.png",&pixels,800,600,image::ColorType::Rgba8).unwrap();
        draw(&world,&r,&mut scene,None);assert!(!scene.instances[second.3].visible);
        draw(&world,&r,&mut scene,Some(s));assert!(scene.instances[second.3].visible);assert_eq!(scene.instances.len(),1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
