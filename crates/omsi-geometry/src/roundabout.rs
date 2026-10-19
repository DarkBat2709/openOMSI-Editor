//! One shared circulating lane, radial two-way approaches and a raised central island.
//! Geometry uses metres in the OMSI XY plane. Existing junction export/placement is reused.
use crate::junction::*;
use glam::{DVec2,DVec3,Mat4,Vec2};
use omsi_o3d::{Mesh,Material};
use serde::{Serialize,Deserialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Roundabout {
    #[serde(default)] pub splitters:[crate::roundabout_splitters::Splitter;4],
    pub island_radius: f64,
    pub road_width: f64,
    pub island_height: f64,
    #[serde(default)] pub outer_line: bool,
    #[serde(default)] pub dashed_entries: bool,
    #[serde(default="default_line_width")] pub line_width: f64,
    #[serde(default="default_line_insets")] pub line_insets: [[f64;2];4],
}
fn default_line_width()->f64 {0.12}
fn default_line_insets()->[[f64;2];4] {[[0.25;2];4]}
impl Default for Roundabout {
    fn default() -> Self {Self {splitters:std::array::from_fn(|_|Default::default()),island_radius:12.0,road_width:6.0,island_height:0.15,outer_line:false,dashed_entries:false,line_width:default_line_width(),line_insets:default_line_insets()}}
}
fn polar(r:f64,a:f64)->DVec3 {DVec3::new(r*a.sin(),r*a.cos(),0.0)}
pub fn delta(a:f64,b:f64)->f64 {(a-b+std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)-std::f64::consts::PI}

/// Boundary of the union of the circular road and non-overlapping straight approaches.
fn flare(outer:f64,half:f64)->(f64,f64) {
    // A short widened mouth gives the entry curve room, without changing the port.
    let slope=2.5/8.0;
    (slope,half+slope*(outer+8.0))
}
pub fn mouth_angle(outer:f64,half:f64)->f64 {
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
pub fn on_road(p:&Project,point:DVec3)->bool {
    let r=p.roundabout.as_ref().unwrap();let radius=point.truncate().length();
    radius>=r.island_radius-1e-6 && radius<=outer_radius(p,point.x.atan2(point.y))+1e-6 && !crate::roundabout_splitters::contains(p,point)
}

// Open boundary strips follow the actual arm flare and circle outline. Port caps
// are excluded entirely, so no stripe can close an entrance.
pub fn edge_strips(p:&Project,outline:&[DVec2])->Vec<[DVec3;4]> {
    let r=p.roundabout.as_ref().unwrap();let n=outline.len();
    let cap=|a:DVec2,b:DVec2|p.arms.iter().any(|arm| {
        if !arm.enabled {return false;}
        let d=direction(arm.angle);let q=right(d);
        [a,b].iter().all(|v|(v.dot(d)-arm.length).abs()<1e-5 && v.dot(q).abs()<=arm.width/2.0+1e-5)
    });
    let open=(0..n).map(|i|!cap(outline[i],outline[(i+1)%n])).collect::<Vec<_>>();
    let inset=|v:DVec2| {
        let outer=r.island_radius+r.road_width;
        let mut value=0.25;
        let mut best=0.0;
        for (i,arm) in p.arms.iter().enumerate().filter(|(_,a)|a.enabled) {
            let d=direction(arm.angle);let along=v.dot(d);
            let t=((along-outer)/(arm.length-outer)).clamp(0.0,1.0);
            if t>best {best=t;let side=if v.dot(right(d))>=0.0 {0}else{1};
                value=0.25+(r.line_insets[i][side]-0.25)*t;
            }
        }
        value
    };
    let vertex=|i:usize,dist:f64| {
        let before=(i+n-1)%n;let after=(i+1)%n;
        let a=right((outline[i]-outline[before]).normalize_or_zero());
        let b=right((outline[after]-outline[i]).normalize_or_zero());
        let normal=if !open[before] {b} else if !open[i] {a} else {
            let bisector=(a+b).normalize_or_zero();bisector/bisector.dot(b).max(0.25)
        };
        (outline[i]+normal*dist).extend(0.015)
    };
    (0..n).filter(|&i|open[i]).map(|i| {
        let j=(i+1)%n;let a=inset(outline[i]);let b=inset(outline[j]);let half=r.line_width/2.0;
        [vertex(i,a-half),vertex(i,a+half),vertex(j,b+half),vertex(j,b-half)]
    }).collect()
}

pub fn entrance_dashes(p:&Project)->Vec<[DVec3;4]> {
    let r=p.roundabout.as_ref().unwrap();let radius=r.island_radius+r.road_width-0.25;
    let half=r.line_width/2.0;let h=DVec3::Z*0.017;let mut result=Vec::new();
    for arm in p.arms.iter().filter(|a|a.enabled) {
        let opening=mouth_angle(r.island_radius+r.road_width,arm.width/2.0);
        let length=2.0*opening*radius;
        // Equal one-metre strokes and gaps, centred on the opening.
        let strokes=((length+1.0)/2.0).floor().max(1.0) as usize;
        let start=(length-(strokes as f64*2.0-1.0))/2.0;
        for i in 0..strokes {for j in 0..4 {
            let a=arm.angle.to_radians()-opening+(start+2.0*i as f64+j as f64/4.0)/radius;
            let b=a+0.25/radius;
            result.push([polar(radius-half,a)+h,polar(radius+half,a)+h,polar(radius+half,b)+h,polar(radius-half,b)+h]);
        }}
    }
    result
}

pub fn build(p:&Project)->Result<Built,String> {
    let r=p.roundabout.as_ref().unwrap();
    if !r.island_radius.is_finite() || !(5.0..=60.0).contains(&r.island_radius)
        || !r.road_width.is_finite() || !(4.0..=12.0).contains(&r.road_width)
        || !r.island_height.is_finite() || !(0.05..=1.0).contains(&r.island_height) {return Err("Invalid roundabout dimensions".into());}
    if !r.line_width.is_finite() || !(0.05..=0.5).contains(&r.line_width)
        || r.line_insets.iter().flatten().any(|v|!v.is_finite() || !(0.05..=3.0).contains(v)) {
        return Err("Invalid edge line dimensions".into());
    }
    if r.outer_line && p.arms.iter().enumerate().any(|(i,a)|a.enabled && r.line_insets[i].iter().any(|v|*v<r.line_width/2.0 || *v+r.line_width/2.0>=a.width/2.0)) {
        return Err("Edge line must stay inside its arm".into());
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
        Material {diffuse:[0.65,0.66,0.64,1.0],..Default::default()},
    ],..Default::default()};
    // Include every entrance corner/intersection angle in the mesh. This prevents
    // triangles shaving off the ends of roads or leaving cracks beside the circle.
    let mut angles:Vec<_>=(0..360).map(|i|i as f64*std::f64::consts::TAU/360.0).collect();
    for a in &arms {for offset in [mouth_angle(outer,a.width/2.0),(a.width/2.0/(outer+8.0)).atan(),(a.width/2.0/a.length).atan()] {for side in [-1.0,1.0] {
        angles.push((a.angle.to_radians()+side*offset).rem_euclid(std::f64::consts::TAU));
    }}}
    angles.sort_by(f64::total_cmp);angles.dedup_by(|a,b|(*a-*b).abs()<1e-9);angles.push(std::f64::consts::TAU);
    let outline:Vec<_>=angles[..angles.len()-1].iter().map(|&a|polar(outer_radius(p,a),a).truncate()).collect();
    if crate::outline_crosses_itself(&outline) {return Err("Roundabout boundary intersects itself".into());}
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
        let inner=r.island_radius-0.15;
        triangle(&mut mesh,z,polar(inner,a)+z,polar(inner,b)+z,1,p.texture_metres);
        // Circle points run clockwise; reverse the edge for an outward-facing wall.
        island_kerb(&mut mesh,polar(r.island_radius,b).truncate(),polar(r.island_radius,a).truncate(),polar(inner,b).truncate(),polar(inner,a).truncate(),r.island_height);
        if p.markings {
            let h=DVec3::Z*0.012;
            quad(&mut mesh,polar(r.island_radius+0.18,a)+h,polar(r.island_radius+0.30,a)+h,
                polar(r.island_radius+0.30,b)+h,polar(r.island_radius+0.18,b)+h,2,1.0);
        }
    }
    if r.outer_line {for [a,b,c,d] in edge_strips(p,&outline) {quad(&mut mesh,a,b,c,d,2,1.0);}}
    if r.dashed_entries {for [a,b,c,d] in entrance_dashes(p) {quad(&mut mesh,a,b,c,d,2,1.0);}}
    // Connections use separate shared ring nodes: every entrance can reach every exit,
    // without duplicating all origin/destination routes or crossing the island.
    let mut entries=Vec::new();let mut exits=Vec::new();let mut nodes=Vec::new();
    for a in &arms {
        let splitter=&r.splitters[p.arms.iter().position(|arm|std::ptr::eq(arm,*a)).unwrap()];
        let angle=a.angle.to_radians();
        let d=direction(a.angle).extend(0.0);let side=right(direction(a.angle)).extend(0.0);
        let offset=a.width/4.0;let radius=(r.road_width*1.5).clamp(6.0,15.0);
        // Circular fillets are tangent to both the approach lane and the ring.
        // Their centres are exactly mid+radius from the roundabout centre.
        let join=((mid+radius).powi(2)-(offset+radius).powi(2)).sqrt();
        if !join.is_finite() || join>a.length {return Err("Entrance too short for a smooth entry curve".into());}
        splitter.validate(a,outer,join)?;
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
            let straight_steps=if splitter.enabled {((a.length-join)/0.25).ceil().max(1.0) as usize}else{1};
            let mut pts:Vec<_>=(0..straight_steps).map(|i|{let along=a.length-(a.length-join)*i as f64/straight_steps as f64;
                d*along-side*(side_sign*(offset+splitter.deflection(outer,along)))}).collect();
            pts.extend((0..=n).map(|i| {
                let (sn,cs)=(turn*i as f64/n as f64).sin_cos();
                let point=centre+DVec3::new(v0.x*cs-v0.y*sn,v0.x*sn+v0.y*cs,0.0);
                point-side*(side_sign*splitter.deflection(outer,point.dot(d)))
            }));
            *pts.last_mut().unwrap()=end;pts
        };
        let incoming=approach(sign,entry);
        let mut outgoing=approach(-sign,exit);outgoing.reverse();
        // Check the lane corridor, not just its centre line, against the raised island
        // and the outer road edge. Invalid settings cannot be exported.
        let width=((a.width-if splitter.enabled{splitter.width}else{0.0})/2.0).min(r.road_width*0.8);
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
    let mut built=Built {mesh,paths,outline};
    crate::roundabout_splitters::add_mesh(p,&mut built);
    Ok(built)
}

