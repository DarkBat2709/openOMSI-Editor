//! One shared circulating lane, radial two-way approaches and a raised central island.
//! Geometry uses metres in the OMSI XY plane. Existing junction export/placement is reused.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Roundabout {
    pub island_radius: f64,
    pub road_width: f64,
    pub island_height: f64,
}
impl Default for Roundabout {
    fn default() -> Self {Self {island_radius:12.0,road_width:6.0,island_height:0.15}}
}
fn polar(r:f64,a:f64)->DVec3 {DVec3::new(r*a.sin(),r*a.cos(),0.0)}
fn delta(a:f64,b:f64)->f64 {(a-b+std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)-std::f64::consts::PI}

/// Boundary of the union of the circular road and non-overlapping straight approaches.
fn flare(outer:f64,half:f64)->(f64,f64) {
    // A short widened mouth gives the entry curve room, without changing the port.
    let slope=2.5/8.0;
    (slope,half+slope*(outer+8.0))
}
fn mouth_angle(outer:f64,half:f64)->f64 {
    let (k,c)=flare(outer,half);
    (c/(outer*(1.0+k*k).sqrt())).clamp(-1.0,1.0).asin()-k.atan()
}
fn outer_radius(p:&Project,angle:f64)->f64 {
    let r=p.roundabout.as_ref().unwrap();let outer=r.island_radius+r.road_width;
    p.arms.iter().filter(|a|a.enabled).fold(outer,|radius,a| {
        let d=delta(angle,a.angle.to_radians());
        if d.cos()<=0.0 {return radius;}
        let straight=(a.length/d.cos()).min(a.width/2.0/d.sin().abs().max(1e-12));
        let (k,c)=flare(outer,a.width/2.0);
        let mouth=((outer+8.0)/d.cos()).min(c/(d.sin().abs()+k*d.cos()));
        radius.max(straight).max(mouth)
    })
}
fn on_road(p:&Project,point:DVec3)->bool {
    let r=p.roundabout.as_ref().unwrap();let radius=point.truncate().length();
    radius>=r.island_radius-1e-6 && radius<=outer_radius(p,point.x.atan2(point.y))+1e-6
}

pub(super) fn build(p:&Project)->Result<Built,String> {
    let r=p.roundabout.as_ref().unwrap();
    for (f,v) in [(Field::IslandRadius,r.island_radius),(Field::RingWidth,r.road_width),(Field::IslandHeight,r.island_height)] {
        let (lo,hi)=f.limits();if !v.is_finite() || !(lo..=hi).contains(&v) {return Err(format!("Invalid value: {}",f.title()));}
    }
    let outer=r.island_radius+r.road_width;let mid=r.island_radius+r.road_width/2.0;
    let sign=if p.left_hand {-1.0} else {1.0};
    let arms:Vec<_>=p.arms.iter().filter(|a|a.enabled).collect();
    for a in &arms {
        if a.bend.abs()>1e-6 || a.sidewalk.abs()>1e-6 {return Err("Roundabout entrances must be straight and without sidewalks".into());}
        if a.length<outer+8.0 {return Err(format!("Entrance length must be at least {:.1} m from the centre",outer+8.0));}
        if a.width>r.road_width*2.0 || a.width/2.0>=outer*0.65 {return Err("Entrance too wide: reduce its width or enlarge the ring".into());}
    }
    for (i,a) in arms.iter().enumerate() {for b in arms.iter().skip(i+1) {
        let separation=delta(a.angle.to_radians(),b.angle.to_radians()).abs();
        let need=mouth_angle(outer,a.width/2.0)+mouth_angle(outer,b.width/2.0)+0.15;
        if separation<need {return Err("Roundabout entrances overlap: change angles or enlarge the ring".into());}
    }}
    let mut mesh=Mesh {version:4,transform:Mat4::IDENTITY,has_transform:true,materials:vec![
        Material {texture:p.road_texture.clone(),diffuse:if p.road_texture.is_empty(){[0.22,0.23,0.24,1.0]}else{[1.0;4]},..Default::default()},
        Material {texture:p.walk_texture.clone(),diffuse:if p.walk_texture.is_empty(){[0.38,0.47,0.24,1.0]}else{[1.0;4]},..Default::default()},
        Material {diffuse:[0.96,0.94,0.84,1.0],..Default::default()},
    ],..Default::default()};
    // Include every entrance corner/intersection angle in the mesh. This prevents
    // triangles shaving off the ends of roads or leaving cracks beside the circle.
    let mut angles:Vec<_>=(0..360).map(|i|i as f64*std::f64::consts::TAU/360.0).collect();
    for a in &arms {for offset in [mouth_angle(outer,a.width/2.0),(a.width/2.0/(outer+8.0)).atan(),(a.width/2.0/a.length).atan()] {for side in [-1.0,1.0] {
        angles.push((a.angle.to_radians()+side*offset).rem_euclid(std::f64::consts::TAU));
    }}}
    angles.sort_by(f64::total_cmp);angles.dedup_by(|a,b|(*a-*b).abs()<1e-9);angles.push(std::f64::consts::TAU);
    let outline:Vec<_>=angles[..angles.len()-1].iter().map(|&a|polar(outer_radius(p,a),a).truncate()).collect();
    if omsi_geometry::outline_crosses_itself(&outline) {return Err("Roundabout boundary intersects itself".into());}
    for pair in angles.windows(2) {
        let (a,b)=(pair[0],pair[1]);
        let start=mesh.vertices.len();
        quad(&mut mesh,polar(r.island_radius,a),polar(outer_radius(p,a),a),polar(outer_radius(p,b),b),polar(r.island_radius,b),0,p.texture_metres);
        if let Some(surface)=&p.road_surface {
            // Polar UVs follow the ring; the seam is confined to one radial edge.
            for v in &mut mesh.vertices[start..] {
                let xy=DVec2::new(v.position.x as f64,v.position.z as f64);
                let mut theta=xy.x.atan2(xy.y).rem_euclid(std::f64::consts::TAU);
                if (theta-a).abs()>(theta-b).abs() {theta=b;}else{theta=a;}
                // At the closing seam use 2π rather than wrapping back to zero.
                if b>6.28 && xy.x.abs()<1e-4 && xy.y>0.0 {theta=b;}
                v.uv=Vec2::new(((xy.length()-r.island_radius)/surface.width_metres) as f32,
                    (theta*mid/p.texture_metres*if surface.reverse_v{-1.0}else{1.0})as f32);
            }
        }
        let z=DVec3::Z*r.island_height;
        triangle(&mut mesh,z,polar(r.island_radius,a)+z,polar(r.island_radius,b)+z,1,p.texture_metres);
        quad(&mut mesh,polar(r.island_radius,a),polar(r.island_radius,b),polar(r.island_radius,b)+z,polar(r.island_radius,a)+z,1,p.texture_metres);
        if p.markings {
            let h=DVec3::Z*0.012;
            quad(&mut mesh,polar(r.island_radius+0.18,a)+h,polar(r.island_radius+0.30,a)+h,
                polar(r.island_radius+0.30,b)+h,polar(r.island_radius+0.18,b)+h,2,1.0);
        }
    }
    // Connections use separate shared ring nodes: every entrance can reach every exit,
    // without duplicating all origin/destination routes or crossing the island.
    let mut entries=Vec::new();let mut exits=Vec::new();let mut nodes=Vec::new();
    for a in &arms {
        let angle=a.angle.to_radians();
        let d=direction(a.angle).extend(0.0);let side=right(direction(a.angle)).extend(0.0);
        let offset=a.width/4.0;let radius=(r.road_width*1.5).clamp(6.0,15.0);
        // Circular fillets are tangent to both the approach lane and the ring.
        // Their centres are exactly mid+radius from the roundabout centre.
        let join=((mid+radius).powi(2)-(offset+radius).powi(2)).sqrt();
        if !join.is_finite() || join>a.length {return Err("Entrance too short for a smooth entry curve".into());}
        let spread=((offset+radius)/(mid+radius)).asin();
        let entry=(angle-sign*spread).rem_euclid(std::f64::consts::TAU);
        let exit=(angle+sign*spread).rem_euclid(std::f64::consts::TAU);
        nodes.extend([entry,exit]);
        let approach=|side_sign:f64,anchor:f64| {
            let centre=d*join-side*(side_sign*(offset+radius));
            let start=d*join-side*(side_sign*offset);
            let end=polar(mid,anchor);let v0=start-centre;let v1=end-centre;
            let turn=v0.truncate().perp_dot(v1.truncate()).atan2(v0.dot(v1));
            let n=(radius*turn.abs()/0.5).ceil().max(4.0)as usize;
            let mut pts=vec![d*a.length-side*(side_sign*offset)];
            pts.extend((0..=n).map(|i| {
                let (sn,cs)=(turn*i as f64/n as f64).sin_cos();
                centre+DVec3::new(v0.x*cs-v0.y*sn,v0.x*sn+v0.y*cs,0.0)
            }));
            *pts.last_mut().unwrap()=end;pts
        };
        let incoming=approach(sign,entry);
        let mut outgoing=approach(-sign,exit);outgoing.reverse();
        // Check the lane corridor, not just its centre line, against the raised island
        // and the outer road edge. Invalid settings cannot be exported.
        let width=(a.width/2.0).min(r.road_width*0.8);
        for pts in [&incoming,&outgoing] {for (i,&point) in pts.iter().enumerate() {
            let tangent=pts[(i+1).min(pts.len()-1)]-pts[i.saturating_sub(1)];
            let lateral=right(tangent.truncate().normalize_or_zero()).extend(0.0)*width*0.46;
            if !on_road(p,point) || !on_road(p,point+lateral) || !on_road(p,point-lateral) {
                return Err("Entry curve leaves the road: enlarge the ring width or reduce entrance width".into());
            }
        }}
        entries.push((incoming,width,1));exits.push((outgoing,width,if p.left_hand {2}else{3}));
    }
    nodes.sort_by(f64::total_cmp);
    if sign>0.0 {nodes.reverse();}
    let mut paths=Vec::new();
    for i in 0..nodes.len() {
        let start=nodes[i];let next=nodes[(i+1)%nodes.len()];
        let arc=if sign>0.0 {(start-next).rem_euclid(std::f64::consts::TAU)}else{(next-start).rem_euclid(std::f64::consts::TAU)};
        let n=(arc*mid/0.75).ceil().max(2.0)as usize;
        let mut pts:Vec<_>=(0..=n).map(|j|polar(mid,start-sign*arc*j as f64/n as f64)).collect();
        // Exactly identical node coordinates also survive the six-decimal SCO export.
        pts[0]=polar(mid,start);*pts.last_mut().unwrap()=polar(mid,next);
        paths.push((pts,r.road_width*0.8,1));
    }
    paths.extend(entries);paths.extend(exits);
    Ok(Built {mesh,paths,outline})
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let mut w=Window::new_roundabout(false);w.project.road_texture="Texture/asphalt.png".into();
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
