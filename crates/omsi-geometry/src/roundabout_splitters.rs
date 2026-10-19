//! Parametric splitter islands. Ports stay fixed; lane centres deflect around the kerbs.
use crate::junction::{Project,Arm,Built,direction,right,triangle,quad,island_kerb};
use glam::{DVec2,DVec3};
use serde::{Serialize,Deserialize};
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
pub enum Shape {#[default] Rounded,Teardrop,Wedge}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Splitter {pub enabled:bool,pub shape:Shape,pub length:f64,pub width:f64,pub gap:f64,pub height:f64}
impl Default for Splitter {fn default()->Self {Self {enabled:false,shape:Shape::Rounded,length:6.0,width:1.5,gap:7.0,height:0.15}}}
impl Splitter {
    pub fn half_width(&self,t:f64)->f64 {
        if !(0.0..=1.0).contains(&t){return 0.0;}
        let r=self.width/2.0;
        match self.shape {
            Shape::Wedge=>{
                let along=t*self.length;let tail=r*0.4;
                if along>=self.length-tail {return (tail*tail-(along-(self.length-tail)).powi(2)).max(0.0).sqrt();}
                let corner=(r*0.15).min(0.25).min(self.length*0.05);
                let rounding=if along<corner {corner-(corner*corner-(corner-along).powi(2)).max(0.0).sqrt()}else{0.0};
                r-(r-tail)*along/(self.length-tail)-rounding
            },
            Shape::Rounded=>{let end=(t*self.length).min((1.0-t)*self.length);if end>=r {r}else{(r*r-(r-end).powi(2)).max(0.0).sqrt()}},
            // t runs outward from the ring: broad shoulder near the ring, taper towards the road.
            Shape::Teardrop=>r*6.75*t*(1.0-t)*(1.0-t),
        }
    }
    pub fn start(&self,outer:f64)->f64 {outer+self.gap}
    pub fn contains(&self,outer:f64,along:f64,lateral:f64)->bool {
        self.enabled && along>self.start(outer) && along<self.start(outer)+self.length
            && lateral.abs()<self.half_width((along-self.start(outer))/self.length)+1e-6
    }
    pub fn deflection(&self,outer:f64,along:f64)->f64 {
        if !self.enabled {return 0.0;}
        let start=self.start(outer);let end=start+self.length;
        let t=((along-(start-2.0))/2.0).min((end+2.0-along)/2.0).clamp(0.0,1.0);
        self.width/4.0*t*t*(3.0-2.0*t)
    }
    pub fn validate(&self,arm:&Arm,outer:f64,join:f64)->Result<(),String> {
        if !self.enabled {return Ok(());}
        for (v,lo,hi) in [(self.length,2.0,30.0),(self.width,0.6,4.0),(self.gap,0.0,50.0),(self.height,0.05,0.5)] {
            if !v.is_finite() || !(lo..=hi).contains(&v) {return Err("Invalid splitter island dimensions".into());}
        }
        if self.length<self.width || (arm.width-self.width)/2.0<2.75 {return Err("Splitter island needs at least 2.75 m of road on each side; reduce island width or widen the arm".into());}
        if (self.shape==Shape::Wedge && self.gap<0.75) || (self.shape!=Shape::Wedge && self.start(outer)<join+2.0) {return Err("Splitter island is too close to the ring; increase its distance".into());}
        if self.start(outer)+self.length+2.0>arm.length {return Err("Splitter island needs more entrance length; shorten the island or lengthen the arm".into());}
        Ok(())
    }
}
pub fn contains(p:&Project,point:DVec3)->bool {
    let r=p.roundabout.as_ref().unwrap();let outer=r.island_radius+r.road_width;
    p.arms.iter().zip(&r.splitters).any(|(a,s)|{let d=direction(a.angle);a.enabled&&s.contains(outer,point.truncate().dot(d),point.truncate().dot(right(d)))})
}
pub fn add_mesh(p:&Project,built:&mut Built) {
    let r=p.roundabout.as_ref().unwrap();let outer=r.island_radius+r.road_width;
    for (a,s) in p.arms.iter().zip(&r.splitters).filter(|(a,s)|a.enabled&&s.enabled) {
        let d=direction(a.angle);let side=right(d);let start=s.start(outer);
        let mut stations:Vec<_>=(0..=64).map(|i|i as f64/64.0).collect();
        if s.shape==Shape::Wedge {let corner=(s.width*0.075).min(0.25).min(s.length*0.05);
            for i in 1..=8 {let t=corner/s.length*i as f64/8.0;stations.extend([t,1.0-s.width*0.2/s.length*i as f64/8.0]);}}
        stations.sort_by(f64::total_cmp);stations.dedup_by(|a,b|(*a-*b).abs()<1e-10);
        let mut border:Vec<DVec2>=stations.iter().map(|&t|d*(start+t*s.length)+side*s.half_width(t)).collect();
        border.extend(stations.iter().rev().map(|&t|d*(start+t*s.length)-side*s.half_width(t)));
        border.dedup_by(|a,b|a.distance_squared(*b)<1e-16);
        if border.first().unwrap().distance_squared(*border.last().unwrap())<1e-16 {border.pop();}
        let center=(d*(start+s.length*0.5)).extend(s.height);
        let kerb=0.15_f64.min(s.width*0.2).min(s.length*0.1);
        let inset=|p:DVec2|{let local=p-center.truncate();center.truncate()+d*(local.dot(d)*(1.0-2.0*kerb/s.length))+side*(local.dot(side)*(1.0-2.0*kerb/s.width))};
        for i in 0..border.len() {
            let (a,b)=(border[i],border[(i+1)%border.len()]);
            let (ia,ib)=(inset(a),inset(b));
            triangle(&mut built.mesh,center,ia.extend(s.height),ib.extend(s.height),1,p.texture_metres);
            island_kerb(&mut built.mesh,a,b,ia,ib,s.height);
            if p.markings {
                let normal=right((b-a).normalize_or_zero());
                quad(&mut built.mesh,(a+normal*0.12).extend(0.016),(b+normal*0.12).extend(0.016),
                    (b+normal*(0.12+r.line_width)).extend(0.016),(a+normal*(0.12+r.line_width)).extend(0.016),2,1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roundabout::{Roundabout,build,on_road};
    fn project()->Project {let mut p=Project::default();p.format=2;p.roundabout=Some(Roundabout::default());for a in &mut p.arms {a.enabled=true;a.length=35.0;}p}
    #[test]
    fn islands_preserve_ports_and_keep_both_traffic_sides_clear() {
        for left in [false,true] {for count in [3,4] {for shape in [Shape::Rounded,Shape::Teardrop,Shape::Wedge] {
            let mut p=project();p.left_hand=left;p.arms[3].enabled=count==4;
            let original=build(&p).unwrap();
            for s in &mut p.roundabout.as_mut().unwrap().splitters {s.enabled=true;s.shape=shape;if shape==Shape::Wedge{s.gap=1.0;s.length=9.0;}}
            let b=build(&p).unwrap();assert_eq!(b.paths.len(),original.paths.len());assert!(b.mesh.triangles.len()>original.mesh.triangles.len());
            for (path,old) in b.paths.iter().zip(&original.paths) {assert_eq!(path.0.first(),old.0.first());assert_eq!(path.0.last(),old.0.last());
                for pair in path.0.windows(2) {let tangent=(pair[1]-pair[0]).truncate().normalize();let side=right(tangent).extend(0.0)*path.1*0.46;
                    for t in [0.0,0.25,0.5,0.75,1.0] {let q=pair[0].lerp(pair[1],t);for v in [q,q+side,q-side]{assert!(on_road(&p,v),"{left} {count} {shape:?}: {v:?}");}}
                }
            }
            for i in count*2..count*3 {assert_ne!(b.paths[i].0,original.paths[i].0);}
            assert!(b.mesh.vertices.iter().all(|v|v.position.is_finite()&&v.normal.is_finite()));
        }}}
    }
    #[test]
    fn teardrop_tapers_towards_the_external_road_on_every_arm() {
        let mut p=project();for a in &mut p.arms {a.angle+=17.0;}
        for s in &mut p.roundabout.as_mut().unwrap().splitters {s.enabled=true;s.shape=Shape::Teardrop;}
        let r=p.roundabout.as_ref().unwrap();let outer=r.island_radius+r.road_width;
        for (a,s) in p.arms.iter().zip(&r.splitters) {
            assert!(s.half_width(0.25)>s.half_width(0.75));
            let d=direction(a.angle);let lateral=right(d)*s.width*0.3;
            assert!(contains(&p,(d*(s.start(outer)+s.length*0.25)+lateral).extend(0.0)));
            assert!(!contains(&p,(d*(s.start(outer)+s.length*0.75)+lateral).extend(0.0)));
        }
        build(&p).unwrap();
    }
    #[test]
    fn wedge_has_broad_ring_face_and_rounded_narrow_road_end() {
        let s=Splitter{shape:Shape::Wedge,enabled:true,gap:1.0,length:9.0,..Default::default()};
        assert!(s.half_width(0.0)>s.half_width(0.9)*1.5);assert_eq!(s.half_width(1.0),0.0);
        assert!(s.half_width(0.01)>s.half_width(0.0));assert!(s.half_width(0.99)>s.half_width(1.0));
        let mut p=project();for a in &mut p.arms {a.angle+=17.0;}
        p.roundabout.as_mut().unwrap().splitters=std::array::from_fn(|_|s.clone());let b=build(&p).unwrap();
        for a in &p.arms {let d=direction(a.angle);let front=d*19.0;let side=right(d)*s.half_width(0.0);
            for xy in [front-side,front+side] {assert!(b.mesh.vertices.iter().any(|v|(v.position.x as f64-xy.x).abs()<1e-5&&(v.position.z as f64-xy.y).abs()<1e-5&&(v.position.y as f64-(s.height-0.02)).abs()<1e-5));}}
        p.roundabout.as_mut().unwrap().splitters[0].gap=0.5;assert!(build(&p).is_err());
    }
    #[test]
    fn concrete_kerbs_have_separate_material_and_outward_faces_without_markings() {
        for shape in [Shape::Rounded,Shape::Teardrop,Shape::Wedge] {
            let mut p=project();p.markings=false;p.walk_texture="grass.bmp".into();
            let island=&mut p.roundabout.as_mut().unwrap().splitters[0];island.enabled=true;island.shape=shape;
            let b=build(&p).unwrap();assert!(b.mesh.materials[3].texture.is_empty());assert_eq!(b.mesh.materials[1].texture,"grass.bmp");
            let mut ring_walls=0;let mut splitter_walls=0;let mut crowns=0;
            let center=direction(p.arms[0].angle)*(18.0+7.0+3.0);
            for tri in b.mesh.triangles.iter().filter(|t|t.material==3) {
                let v=&b.mesh.vertices[tri.indices[0] as usize];
                if v.normal.y.abs()<0.01 {
                    let xy=DVec2::new(v.position.x as f64,v.position.z as f64);let n=DVec2::new(v.normal.x as f64,v.normal.z as f64);
                    if xy.length()<12.01 {assert!(n.dot(xy)>0.0);ring_walls+=1;}else{assert!(n.dot(xy-center)>-1e-6);splitter_walls+=1;}
                }else{assert!(v.normal.y>0.0);crowns+=1;}
            }
            assert!(ring_walls>100&&splitter_walls>100&&crowns>100);
            assert!(!b.mesh.triangles.iter().any(|t|t.material==2));
        }
    }
    #[test]
    fn rejects_insufficient_space_and_nonfinite_dimensions() {
        let p=project();
        for (field,value) in [(0,29.0),(1,4.0),(2,0.0),(3,f64::NAN)] {
            let mut p=p.clone();let s=&mut p.roundabout.as_mut().unwrap().splitters[0];s.enabled=true;
            match field {0=>s.length=value,1=>s.width=value,2=>s.gap=value,_=>s.height=value}
            assert!(build(&p).is_err());
        }
        let mut p=p;let s=&mut p.roundabout.as_mut().unwrap().splitters[3];s.enabled=true;s.length=30.0;
        p.arms[3].enabled=false;assert!(build(&p).is_ok(),"disabled arms must not constrain geometry");
    }
    #[test]
    fn dimensions_and_shape_change_mesh_and_are_local_to_one_arm() {
        let mut p=project();let original=build(&p).unwrap();p.roundabout.as_mut().unwrap().splitters[0].enabled=true;
        let a=build(&p).unwrap();let s=&mut p.roundabout.as_mut().unwrap().splitters[0];s.length=5.0;s.width=1.0;s.height=0.25;s.shape=Shape::Teardrop;
        let b=build(&p).unwrap();assert_ne!(a.mesh.vertices,b.mesh.vertices);assert_ne!(a.paths[8].0,b.paths[8].0);
        for i in [9,10,11,13,14,15]{assert_eq!(a.paths[i],original.paths[i]);}
        p.roundabout.as_mut().unwrap().splitters[0].enabled=false;assert_eq!(build(&p).unwrap().mesh.vertices,original.mesh.vertices);
    }
}
