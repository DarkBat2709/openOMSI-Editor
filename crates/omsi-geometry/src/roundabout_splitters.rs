//! Parametric splitter islands. Ports stay fixed; lane centres deflect around the kerbs.
use crate::junction::{Project,Arm,Built,direction,right,triangle,quad};
use glam::{DVec2,DVec3};
use serde::{Serialize,Deserialize};
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
pub enum Shape {#[default] Rounded,Teardrop}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Splitter {pub enabled:bool,pub shape:Shape,pub length:f64,pub width:f64,pub gap:f64,pub height:f64}
impl Default for Splitter {fn default()->Self {Self {enabled:false,shape:Shape::Rounded,length:6.0,width:1.5,gap:7.0,height:0.15}}}
impl Splitter {
    pub fn half_width(&self,t:f64)->f64 {
        if !(0.0..=1.0).contains(&t){return 0.0;}
        let r=self.width/2.0;
        match self.shape {
            Shape::Rounded=>{let end=(t*self.length).min((1.0-t)*self.length);if end>=r {r}else{(r*r-(r-end).powi(2)).max(0.0).sqrt()}},
            Shape::Teardrop=>r*6.75*t*t*(1.0-t),
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
        if self.start(outer)<join+2.0 {return Err("Splitter island is too close to the ring; increase its distance".into());}
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
        let mut border:Vec<DVec2>=(0..=64).map(|i|{let t=i as f64/64.0;d*(start+t*s.length)+side*s.half_width(t)}).collect();
        border.extend((1..64).rev().map(|i|{let t=i as f64/64.0;d*(start+t*s.length)-side*s.half_width(t)}));
        let center=(d*(start+s.length*0.5)).extend(s.height);
        for i in 0..border.len() {
            let (a,b)=(border[i],border[(i+1)%border.len()]);
            triangle(&mut built.mesh,center,a.extend(s.height),b.extend(s.height),1,p.texture_metres);
            quad(&mut built.mesh,a.extend(0.0),b.extend(0.0),b.extend(s.height),a.extend(s.height),1,p.texture_metres);
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
        for left in [false,true] {for count in [3,4] {for shape in [Shape::Rounded,Shape::Teardrop] {
            let mut p=project();p.left_hand=left;p.arms[3].enabled=count==4;
            let original=build(&p).unwrap();
            for s in &mut p.roundabout.as_mut().unwrap().splitters {s.enabled=true;s.shape=shape;}
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
