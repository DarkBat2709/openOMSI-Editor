//! Independent circulating/approach lane counts, with shared nodes and explicit lane changes.
use crate::junction::{Project,direction,right,quad};
use crate::roundabout::on_road;
use glam::DVec3;
use omsi_o3d::Mesh;
type Path=(Vec<DVec3>,f64,i32);
fn polar(r:f64,a:f64)->DVec3 {DVec3::new(r*a.sin(),r*a.cos(),0.0)}
fn arc(a:f64,b:f64,sign:f64)->f64 {((a-b)*sign).rem_euclid(std::f64::consts::TAU)}
fn corridor(p:&Project,points:&[DVec3],width:f64)->bool {
    points.windows(2).all(|pair|{let side=right((pair[1]-pair[0]).truncate().normalize_or_zero()).extend(0.0)*width*0.46;
        [0.0,0.5,1.0].iter().all(|&t|{let point=pair[0].lerp(pair[1],t);[point,point+side,point-side].iter().all(|&v|on_road(p,v))})})
}
pub fn paths(p:&Project)->Result<Vec<Path>,String> {
    let r=p.roundabout.as_ref().unwrap();let outer=r.island_radius+r.road_width;let sign=if p.left_hand{-1.0}else{1.0};
    let count=r.lanes as usize;let lane_width=r.road_width/count as f64;
    let radii:Vec<_>=(0..count).map(|i|outer-(i as f64+0.5)*lane_width).collect();
    let mut nodes=Vec::new();let mut entries=Vec::new();let mut exits=Vec::new();
    for (arm_index,a) in p.arms.iter().enumerate().filter(|(_,a)|a.enabled) {
        let s=&r.splitters[arm_index];let n=r.approach_lanes[arm_index] as usize;
        let width=(a.width-if s.enabled{s.width}else{0.0})/(2*n) as f64;
        if width<2.75 {return Err("Each lane needs at least 2.75 m width".into());}
        let d=direction(a.angle).extend(0.0);let side=right(direction(a.angle)).extend(0.0);
        for lane in 0..n {
            // Count from outside to inside; differing counts merge/split at shared ring nodes.
            let ring_lane=(lane*count/n).min(count-1);let mid=radii[ring_lane];
            let offset=a.width/2.0-(lane as f64+0.5)*a.width/(2*n) as f64;
            let radius=(lane_width*1.5).clamp(6.0,15.0);
            let join=((mid+radius).powi(2)-(offset+radius).powi(2)).sqrt();
            if !join.is_finite() || join>a.length {return Err("Entrance too short for a smooth entry curve".into());}
            s.validate(a,outer,join)?;
            let spread=((offset+radius)/(mid+radius)).asin();
            let entry=(a.angle.to_radians()-sign*spread).rem_euclid(std::f64::consts::TAU);
            let exit=(a.angle.to_radians()+sign*spread).rem_euclid(std::f64::consts::TAU);
            nodes.extend([entry,exit]);
            let approach=|side_sign:f64,anchor:f64| {
                let centre=d*join-side*(side_sign*(offset+radius));let start=d*join-side*(side_sign*offset);
                let end=polar(mid,anchor);let v0=start-centre;let v1=end-centre;
                let turn=v0.truncate().perp_dot(v1.truncate()).atan2(v0.dot(v1));
                let steps=(radius*turn.abs()/0.25).ceil().max(4.0) as usize;
                let straight=((a.length-join)/0.25).ceil().max(1.0) as usize;
                let deflect=|v:DVec3|v-side*(side_sign*s.deflection(outer,v.dot(d))*2.0*(1.0-offset/(a.width/2.0)));
                let mut points:Vec<_>=(0..straight).map(|i|deflect(d*(a.length-(a.length-join)*i as f64/straight as f64)-side*(side_sign*offset))).collect();
                points.extend((0..=steps).map(|i|{let (sn,cs)=(turn*i as f64/steps as f64).sin_cos();deflect(centre+DVec3::new(v0.x*cs-v0.y*sn,v0.x*sn+v0.y*cs,0.0))}));
                *points.last_mut().unwrap()=end;points
            };
            let incoming=approach(sign,entry);let mut outgoing=approach(-sign,exit);outgoing.reverse();
            let path_width=width.min(lane_width)*0.8;
            if !corridor(p,&incoming,path_width)||!corridor(p,&outgoing,path_width){return Err("Entry curve leaves the road: enlarge the ring width or reduce entrance width".into());}
            entries.push((incoming,path_width,1));exits.push((outgoing,path_width,if p.left_hand{2}else{3}));
        }
    }
    nodes.sort_by(f64::total_cmp);nodes.dedup_by(|a,b|(*a-*b).abs()<1e-9);if sign>0.0 {nodes.reverse();}
    let mut paths=Vec::new();
    for (lane,&radius) in radii.iter().enumerate(){for i in 0..nodes.len(){
        let start=nodes[i];let next=nodes[(i+1)%nodes.len()];let angle=arc(start,next,sign);
        let steps=(angle*radius/0.5).ceil().max(2.0) as usize;
        let mut pts:Vec<_>=(0..=steps).map(|j|polar(radius,start-sign*angle*j as f64/steps as f64)).collect();
        pts[0]=polar(radius,start);*pts.last_mut().unwrap()=polar(radius,next);paths.push((pts,lane_width*0.8,1));
        // Smooth changes to adjacent lanes let every approach reach every exit.
        for target in [lane.checked_sub(1),if lane+1<count{Some(lane+1)}else{None}].into_iter().flatten(){
            let end_index=(1..nodes.len()).map(|k|(i+k)%nodes.len()).find(|&j|arc(start,nodes[j],sign)*radius>=25.0);
            if let Some(j)=end_index {let angle=arc(start,nodes[j],sign);let steps=(angle*radius/0.5).ceil().max(2.0) as usize;
                let mut pts:Vec<_>=(0..=steps).map(|k|{let t=k as f64/steps as f64;polar(radius+(radii[target]-radius)*t*t*(3.0-2.0*t),start-sign*angle*t)}).collect();
                pts[0]=polar(radius,start);*pts.last_mut().unwrap()=polar(radii[target],nodes[j]);
                if !corridor(p,&pts,lane_width*0.8){return Err("Lane transition leaves the circulating road".into());}
                paths.push((pts,lane_width*0.8,1));
            }
        }
    }}
    paths.extend(entries);paths.extend(exits);Ok(paths)
}
pub fn markings(p:&Project,mesh:&mut Mesh) {
    if !p.roundabout.as_ref().unwrap().lane_lines{return;}
    let r=p.roundabout.as_ref().unwrap();let outer=r.island_radius+r.road_width;
    let mut stripe=|a:DVec3,b:DVec3|{let side=right((b-a).truncate().normalize_or_zero()).extend(0.0)*r.line_width/2.0;let z=DVec3::Z*0.019;quad(mesh,a-side+z,a+side+z,b+side+z,b-side+z,2,1.0);};
    for lane in 1..r.lanes {let radius=r.island_radius+r.road_width*lane as f64/r.lanes as f64;let length=std::f64::consts::TAU*radius;
        let mut pos=0.0;while pos+2.0<length {for k in 0..8 {stripe(polar(radius,(pos+k as f64*0.25)/radius),polar(radius,(pos+(k+1) as f64*0.25)/radius));}pos+=4.0;}}
    for (i,a) in p.arms.iter().enumerate().filter(|(_,a)|a.enabled){let n=r.approach_lanes[i];let d=direction(a.angle).extend(0.0);let side=right(direction(a.angle)).extend(0.0);
        for lane in 1..n {for sign in [-1.0,1.0] {let offset=a.width/2.0*lane as f64/n as f64;let mut pos=outer+8.0;
            while pos+2.0<a.length {let point=|along:f64|d*along+side*sign*(offset+r.splitters[i].deflection(outer,along)*2.0*(1.0-offset/(a.width/2.0)));
                stripe(point(pos),point(pos+2.0));pos+=4.0;
            }
        }}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn project(ring:u8,approach:u8,left:bool)->Project {
        let mut p=Project::default();p.format=2;p.left_hand=left;
        p.roundabout=Some(crate::roundabout::Roundabout{lanes:ring,approach_lanes:[approach;4],island_radius:30.0,road_width:(ring as f64*6.0).max(approach as f64*3.5),..Default::default()});
        for a in &mut p.arms {a.enabled=true;a.width=7.0*approach as f64;a.length=90.0;}p
    }
    #[test]
    fn lane_markings_toggle_without_changing_paths(){
        let mut p=project(3,2,false);let a=crate::roundabout::build(&p).unwrap();
        p.roundabout.as_mut().unwrap().lane_lines=false;let b=crate::roundabout::build(&p).unwrap();
        assert_eq!(a.paths,b.paths);assert!(a.mesh.triangles.len()>b.mesh.triangles.len());
    }
    #[test]
    fn mixed_approaches_with_splitter_islands_stay_clear() {
        for left in [false,true] {for ring in 1..=4 {for shape in [crate::roundabout_splitters::Shape::Rounded,crate::roundabout_splitters::Shape::Teardrop,crate::roundabout_splitters::Shape::Wedge] {
            let mut p=project(ring,4,left);p.roundabout.as_mut().unwrap().road_width=p.roundabout.as_ref().unwrap().road_width.max(15.0);p.roundabout.as_mut().unwrap().approach_lanes=[1,2,3,4];
            for (i,a) in p.arms.iter_mut().enumerate(){a.width=7.0*(i+1) as f64+2.0;}
            for s in &mut p.roundabout.as_mut().unwrap().splitters{s.enabled=true;s.shape=shape;s.width=2.0;s.length=10.0;s.gap=if shape==crate::roundabout_splitters::Shape::Wedge{1.0}else{15.0};}
            let b=crate::roundabout::build(&p).unwrap_or_else(|e|panic!("mixed {ring} {shape:?}: {e}"));
            assert!(b.paths.iter().all(|(pts,w,_)|corridor(&p,pts,*w)));
        }}}
        let mut p=project(4,4,false);p.arms[0].width=7.0;assert!(crate::roundabout::build(&p).is_err());
    }
    #[test]
    fn all_lane_counts_have_ports_and_connected_paths() {
        for left in [false,true]{for ring in 1..=4 {for n in 1..=4 {
            let p=project(ring,n,left);let b=crate::roundabout::build(&p).unwrap_or_else(|e|panic!("ring={ring} approach={n} left={left}: {e}"));
            let ports=b.paths.iter().filter(|(pts,_,_)|pts.first().unwrap().truncate().length()>80.0 || pts.last().unwrap().truncate().length()>80.0).count();assert_eq!(ports,8*n as usize);
            assert!(b.paths.iter().all(|(pts,w,_)|corridor(&p,pts,*w)));
            // Every incoming lane can reach every outgoing lane through exact exported nodes.
            let key=|v:DVec3|((v.x*1e6).round() as i64,(v.y*1e6).round() as i64);
            let mut edges=std::collections::HashMap::<_,Vec<_>>::new();
            for (pts,_,_) in &b.paths {edges.entry(key(pts[0])).or_default().push(key(*pts.last().unwrap()));}
            for (pts,_,_) in b.paths.iter().filter(|(pts,_,_)|pts[0].truncate().length()>80.0){
                let mut seen=std::collections::HashSet::new();let mut pending=vec![key(pts[0])];while let Some(q)=pending.pop(){if seen.insert(q){if let Some(next)=edges.get(&q){pending.extend(next);}}}
                for (out,_,_) in b.paths.iter().filter(|(pts,_,_)|pts.last().unwrap().truncate().length()>80.0){assert!(seen.contains(&key(*out.last().unwrap())),"unreachable exit {ring}/{n}");}
            }
        }}}
    }
    #[test]
    fn old_projects_default_to_one_lane_and_bad_counts_are_rejected(){
        let p=project(1,1,false);
        for bad in [0,5,255]{let mut p=p.clone();p.roundabout.as_mut().unwrap().lanes=bad;assert!(crate::roundabout::build(&p).is_err());}
    }
}
