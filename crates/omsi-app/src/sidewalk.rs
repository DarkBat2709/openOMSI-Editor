//! Sidewalk placement uses the selected SLI unchanged: materials and UV definitions stay in that file.
use crate::{scene::World,spline_editor::Key};
use glam::{DVec2,DVec3};
use omsi_map::MapSpline;
use omsi_geometry::SplineCurve;

#[derive(Clone,Debug)]
pub struct Settings {pub sides:u8,pub connected:bool,pub start:f64,pub length:f64,pub margin:f64,pub height:f64,pub mirror:bool,pub detach:bool}
impl Default for Settings {fn default()->Self {Self {sides:2,connected:false,start:0.0,length:20.0,margin:0.0,height:0.0,mirror:false,detach:false}}}
#[derive(Clone)]
pub struct Road {pub key:Key,pub spline:MapSpline,pub backwards:bool,pub def:omsi_scenery::Spline}
#[derive(Default)]
pub struct Plan {pub pieces:Vec<((i32,i32),MapSpline)>,pub markers:Vec<DVec3>,pub handles:Vec<DVec3>,pub total:f64,pub actual:f64}
pub fn bounds(def:&omsi_scenery::Spline,mirror:bool)->Result<[(f64,f64);2],String> {
    let mut lo=(f64::INFINITY,0.0f64);let mut hi=(f64::NEG_INFINITY,0.0f64);
    for p in def.profiles.iter().flat_map(|p|&p.points) {let x=p.x as f64*if mirror {-1.0} else {1.0};let z=p.z as f64;
        if !x.is_finite()||!z.is_finite() {return Err("Invalid spline profile".into());}
        if x<lo.0 || (x==lo.0 && z<lo.1) {lo=(x,z);}if x>hi.0 || (x==hi.0 && z<hi.1) {hi=(x,z);}
    }
    if hi.0-lo.0<0.05 || hi.0-lo.0>50.0 || !lo.0.is_finite()||!hi.0.is_finite() {return Err("Profile needs two visible side edges".into());}Ok([lo,hi])
}
fn curve(road:&Road)->SplineCurve {SplineCurve::from_map(&road.spline,DVec2::new(road.key.0.0 as f64,road.key.0.1 as f64)*omsi_map::tile_size()).with_sli(&road.def)}
fn frame(road:&Road,s:f64,side:u8,margin:f64,height:f64)->Result<(DVec3,DVec2),String> {
    let c=curve(road);let at=if road.backwards {road.spline.length-s} else {s};
    let d=SplineCurve::dir(c.heading_at(at))*if road.backwards {-1.0} else {1.0};let right=DVec2::new(d.y,-d.x);
    let edge=bounds(&road.def,road.spline.mirror)?;
    let physical_left=(side==1)^road.backwards;let (x,z)=edge[if physical_left {0} else {1}];
    let p=omsi_geometry::spline_profile_point(&road.def,&c,false,at,x,z)
        +right.extend(0.0)*margin*if side==1 {-1.0} else {1.0}+DVec3::Z*height;
    Ok((p,right))
}
fn tile(p:DVec3)->(i32,i32) {let size=omsi_map::tile_size();((p.x/size).floor() as i32,(p.y/size).floor() as i32)}
/// Construct exact parallel circular arcs for ordinary splines. For skewed or tapered roads,
/// short sections get endpoint corrections from the same profile geometry as the renderer.
pub fn build(roads:&[Road],def:&omsi_scenery::Spline,file:&str,settings:&Settings,texture_offset:f64)->Result<Plan,String> {
    if file.is_empty() || def.only_editor {return Err("Select a visible sidewalk profile".into());}
    if !(1..=3).contains(&settings.sides) || ![settings.start,settings.length,settings.margin,settings.height,texture_offset].iter().all(|v|v.is_finite())
        || settings.start<0.0 || settings.length<0.5 || !(0.0..=20.0).contains(&settings.margin) || settings.height.abs()>10.0 {return Err("Invalid start, length, edge distance or height".into());}
    let total=roads.iter().map(|r|r.spline.length).sum::<f64>();
    if !total.is_finite() || settings.start+settings.length>total+1e-6 {return Err(format!("Range ends beyond the road ({total:.2} m); reduce start/length"));}
    let mut plan=Plan {total,..Default::default()};
    for side in [1,2] {if settings.sides&side==0 {continue;}
        let mirrored=settings.mirror^(side==1);let wb=bounds(def,mirrored)?;
        let near=wb[if side==1 {1} else {0}];
        let mut accumulated=0.0;let mut offset=texture_offset;let mut previous:Option<DVec3>=None;
        for road in roads {
            let from=(settings.start-accumulated).max(0.0);let to=(settings.start+settings.length-accumulated).min(road.spline.length);
            accumulated+=road.spline.length;if to<=from {continue;}
            if !road.spline.length.is_finite() || road.spline.length<=0.0 {return Err("Invalid road length".into());}
            let eb=bounds(&road.def,road.spline.mirror)?;
            let ex=eb[if (side==1)^road.backwards {0} else {1}].0*if road.backwards {-1.0} else {1.0};
            let lateral=ex+settings.margin*if side==1 {-1.0} else {1.0}-near.0;
            let radius=road.spline.radius*if road.backwards {-1.0} else {1.0};
            let factor=if radius.abs()<1e-8 {1.0} else {1.0-lateral/radius};
            if !factor.is_finite() || factor<0.1 || (radius.abs()>1e-8 && (radius-lateral).abs()<(wb[1].0-wb[0].0)+1.0) {return Err("Inner curve too tight for this sidewalk profile".into());}
            let ordinary=road.spline.profile_transitions.iter().all(Option::is_none)&&road.spline.skew_start.abs()<1e-8&&road.spline.skew_end.abs()<1e-8;
            let step=if ordinary {20.0f64.min(10.0/factor)} else {0.5};
            let n=((to-from)/step).ceil().max(1.0) as usize;
            if plan.pieces.len()+n>4000 {return Err("More than 4000 sections; shorten sidewalk range".into());}
            for i in 0..n {
                let a=from+(to-from)*i as f64/n as f64;let b=from+(to-from)*(i+1) as f64/n as f64;
                let at=|s:f64|->Result<(DVec3,DVec2),String>{let (p,right)=frame(road,s,side,settings.margin,settings.height)?;Ok((p-right.extend(0.0)*near.0-DVec3::Z*near.1,right))};
                let (p,ra)=at(a)?;let (q,rb)=at(b)?;
                let anchor_a=p+ra.extend(0.0)*near.0+DVec3::Z*near.1;
                if let Some(prev)=previous {if prev.distance(anchor_a)>0.025 {return Err("Road edges do not meet at a connection; connect the road first".into());}}
                let owner=tile(at((a+b)*0.5)?.0);let o=DVec2::new(owner.0 as f64,owner.1 as f64)*omsi_map::tile_size();
                let length=if ordinary {(b-a)*factor} else {(q-p).truncate().length()};
                if length<0.001 {return Err("Sidewalk section too short".into());}
                let heading=if ordinary {(-ra.y).atan2(ra.x).to_degrees()} else {(q.x-p.x).atan2(q.y-p.y).to_degrees()};
                let dz=|s:f64|->Result<f64,String>{let lo=(s-0.01).max(0.0);let hi=(s+0.01).min(road.spline.length);Ok((at(hi)?.0.z-at(lo)?.0.z)/(hi-lo).max(1e-9)/factor*100.0)};
                let mut part=MapSpline {file:file.into(),pos:[p.x-o.x,p.y-o.y,p.z],heading:heading.rem_euclid(360.0),length,
                    radius:if ordinary && radius.abs()>1e-8 {radius-lateral} else {0.0},grad_start:dz(a)?,grad_end:dz(b)?,delta_h:Some(q.z-p.z),is_h:true,
                    mirror:mirrored,tex_offset:offset,map_chain_offset:Some(offset),..Default::default()};
                if !ordinary {
                    let pc=SplineCurve::from_map(&part,o);
                    for (e,(pos,right)) in [(p,ra),(q,rb)].into_iter().enumerate() {
                        let station=if e==0 {0.0} else {length};let forward=SplineCurve::dir(pc.heading_at(station));let local_right=DVec2::new(forward.y,-forward.x);
                        let offsets=wb.map(|(x,z)| {let goal=pos+right.extend(0.0)*x+DVec3::Z*z;
                            let actual=omsi_geometry::spline_profile_point(def,&pc,false,station,x,z);let d=goal-actual;
                            [d.truncate().dot(local_right),d.truncate().dot(forward),d.z]});
                        let t=omsi_map::ProfileTransition {station,span:length,x:[wb[0].0,wb[1].0],offsets};if !t.valid() {return Err("Edge correction outside valid range".into());}part.profile_transitions[e]=Some(t);
                    }
                }
                let pc=SplineCurve::from_map(&part,o);
                for t in [0.25,0.5,0.75]{let (centre,right)=at(a+(b-a)*t)?;
                    for (x,z) in wb {let want=centre+right.extend(0.0)*x+DVec3::Z*z;let got=omsi_geometry::spline_profile_point(def,&pc,false,length*t,x,z);
                        if want.distance(got)>0.025{return Err("Road transition too distorted; choose a shorter or smoother road range".into());}}
                }
                let count=(length/2.0).ceil().max(1.0) as usize;
                for j in 0..=count {for (x,z) in wb {plan.markers.push(omsi_geometry::spline_profile_point(def,&pc,false,length*j as f64/count as f64,x,z));}}
                if plan.handles.is_empty() {plan.handles.push(anchor_a);}
                let anchor_b=q+rb.extend(0.0)*near.0+DVec3::Z*near.1;
                if side==if settings.sides&1!=0 {1} else {2} {if plan.handles.len()==1 {plan.handles.push(anchor_b);}else {plan.handles[1]=anchor_b;}}
                previous=Some(anchor_b);offset+=length;plan.actual+=length;plan.pieces.push((owner,part));
            }
        }
        // Mark side-chain boundaries until IDs are allocated during confirmation.
        if let Some((_,last))=plan.pieces.last_mut() {last.next_id=-1;}
    }
    if plan.pieces.is_empty() {return Err("No sidewalk range selected".into());}Ok(plan)
}
pub fn roads(world:&World,start:Key,connected:bool)->Result<Vec<Road>,String> {
    crate::roadside_objects::sidewalk_route(world,start,connected)?.into_iter().map(|(key,spline,backwards)|{
        let def=world.spline_type(&spline.file).ok_or("Road profile missing")?.def.clone();Ok(Road {key,spline,backwards,def})}).collect()
}
pub fn project_station(roads:&[Road],point:DVec3)->Option<f64> {
    let mut base=0.0;let mut best=(f64::INFINITY,0.0);
    for road in roads {let c=curve(road);let n=(road.spline.length/0.5).ceil().max(1.0) as usize;
        for i in 0..n {let a=i as f64/n as f64*road.spline.length;let b=(i+1) as f64/n as f64*road.spline.length;
            let pa=c.point_at(if road.backwards {road.spline.length-a} else {a}).truncate();let pb=c.point_at(if road.backwards {road.spline.length-b} else {b}).truncate();
            let d=pb-pa;let f=((point.truncate()-pa).dot(d)/d.length_squared().max(1e-10)).clamp(0.0,1.0);let distance=(point.truncate()-pa-d*f).length_squared();
            if distance<best.0 {best=(distance,base+a+(b-a)*f);}}
        base+=road.spline.length;
    }(best.0<2500.0).then_some(best.1)
}
#[derive(Clone,Copy,PartialEq)] pub enum Field {Start,Length,Margin,Height}
impl Field {pub fn title(self)->&'static str {match self {Self::Start=>"Start on road (m)",Self::Length=>"Length along road (m)",Self::Margin=>"Distance from edge (m)",Self::Height=>"Height offset (m)"}}}
#[derive(Clone,Copy)] pub enum Command {Close,Catalog,Existing,New,PickRoad,Edit(Field),Adjust(Field,f64),Sides(u8),Connected(bool),Mirror,Detach,Preview,Apply,Undo,Save}
pub struct Input {pub field:Field,pub text:String,pub replace:bool}
pub struct Window {pub start:Option<Key>,pub existing:Option<Key>,pub file:String,pub settings:Settings,pub input:Option<Input>,pub message:String,pub error:Option<String>,pub preview:Plan,pub route:Vec<Road>,pub rects:Vec<([f32;4],Command)>,pub rect:Option<[f32;4]>,pub picking:u8,pub drag:Option<bool>,pub can_undo:bool}
impl Window {
    pub fn new(start:Option<Key>)->Self {Self {start,existing:None,file:String::new(),settings:Settings::default(),input:None,message:"Choose a sidewalk profile or click an existing sidewalk. Drag the start/end of the blue preview.".into(),error:None,preview:Plan::default(),route:Vec::new(),rects:Vec::new(),rect:None,picking:0,drag:None,can_undo:false}}
    pub fn hit(&self,p:(f32,f32))->Option<Command>{self.rects.iter().rev().find(|(r,_)|p.0>=r[0]&&p.0<=r[2]&&p.1>=r[1]&&p.1<=r[3]).map(|(_,c)|*c)}
    pub fn contains(&self,p:(f32,f32))->bool{self.rect.is_some_and(|r|p.0>=r[0]&&p.0<=r[2]&&p.1>=r[1]&&p.1<=r[3])}
    pub fn value(&self,f:Field)->f64{match f{Field::Start=>self.settings.start,Field::Length=>self.settings.length,Field::Margin=>self.settings.margin,Field::Height=>self.settings.height}}
    pub fn edit(&mut self,f:Field){self.input=Some(Input {field:f,text:format!("{:.2}",self.value(f)),replace:true});}
    pub fn set(&mut self,f:Field,v:f64)->Result<(),String>{let(lo,hi)=match f{Field::Start=>(0.0,100000.0),Field::Length=>(0.5,100000.0),Field::Margin=>(0.0,20.0),Field::Height=>(-10.0,10.0)};
        if !v.is_finite()||v<lo||v>hi{return Err(format!("Value must be between {lo} and {hi}"));}match f{Field::Start=>self.settings.start=v,Field::Length=>self.settings.length=v,Field::Margin=>self.settings.margin=v,Field::Height=>self.settings.height=v}Ok(())}
    pub fn commit(&mut self)->bool {let Some(i)=&self.input else{return true;};let f=i.field;let v=i.text.replace(',',".").parse::<f64>();match v.map_err(|_|"Invalid number".to_string()).and_then(|v|self.set(f,v)){Ok(())=>{self.input=None;true},Err(e)=>{self.message=e;false}}}
    pub fn drag_to(&mut self,station:f64){
        if !station.is_finite(){return;}
        if self.drag==Some(true){let end=self.settings.start+self.settings.length;self.settings.start=station.clamp(0.0,(end-0.5).max(0.0));self.settings.length=end-self.settings.start;}
        else if self.drag==Some(false){self.settings.length=(station-self.settings.start).clamp(0.5,(self.preview.total-self.settings.start).max(0.5));}
    }
    pub fn refresh(&mut self,world:&World){self.preview=Plan::default();self.route.clear();let result=(||{
        let start=self.start.ok_or("Select road")?;self.route=roads(world,start,self.settings.connected)?;
        let total=self.route.iter().map(|r|r.spline.length).sum();self.preview.total=total;
        let def=world.spline_type(&self.file).ok_or("Choose sidewalk profile in catalogue or an existing sidewalk")?;
        let original=self.existing.and_then(|k|world.spline_edits.lock().current(k));
        if self.existing.is_some() && original.is_none(){return Err("Existing sidewalk no longer available".into());}
        if self.existing.is_some() && self.settings.sides==3{return Err("Align existing sidewalk on exactly one side".into());}
        if self.existing.is_some_and(|key|self.route.iter().any(|r|r.key==key)){return Err("Sidewalk and reference road must differ".into());}
        self.preview=build(&self.route,&def.def,&self.file,&self.settings,original.map_or(0.0,|s|s.tex_offset))?;Ok::<(),String>(())})();self.error=result.err();}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(left:f32,right:f32)->omsi_scenery::Spline {
        omsi_scenery::Spline {textures:vec![omsi_scenery::sli::SplineTexture {file:"walk.dds".into(),..Default::default()}],profiles:vec![omsi_scenery::SplineProfile {texture:0,points:vec![
            omsi_scenery::SplineProfilePoint{x:left,z:0.0,u:0.2,v_scale:0.5},
            omsi_scenery::SplineProfilePoint{x:right,z:0.0,u:0.8,v_scale:0.5}]}],..Default::default()}
    }
    fn road(radius:f64)->Road {Road{key:((0,0),1),spline:MapSpline {length:80.0,radius,pos:[100.0,100.0,2.0],heading:37.0,..Default::default()},backwards:false,def:profile(-4.0,4.0)}}
    fn point(tile:(i32,i32),s:&MapSpline,at:f64,x:f64)->DVec3 {
        let c=SplineCurve::from_map(s,DVec2::new(tile.0 as f64,tile.1 as f64)*omsi_map::tile_size());
        omsi_geometry::spline_profile_point(&profile(0.0,2.0),&c,false,at,x,0.0)
    }
    #[test] fn partial_curves_both_sides_keep_edges_and_texture_phase(){
        let def=profile(0.0,2.0);let original=def.clone();
        for radius in [0.0,40.0,-40.0] {for backwards in [false,true]{for side in [1,2]{
            let mut r=road(radius);r.backwards=backwards;
            let settings=Settings {sides:side,start:12.0,length:23.0,margin:0.3,height:0.15,..Default::default()};
            let plan=build(&[r.clone()],&def,"Splines/test/walk.sli",&settings,7.25).unwrap();
            assert!((plan.total-80.0).abs()<1e-9);assert_eq!(plan.handles.len(),2);
            assert!(plan.handles[0].distance(frame(&r,12.0,side,0.3,0.15).unwrap().0)<1e-7);
            assert!(plan.handles[1].distance(frame(&r,35.0,side,0.3,0.15).unwrap().0)<1e-7);
            let mut offset=7.25;let mut last=None;
            for (tile,s) in &plan.pieces {
                assert_eq!(s.file,"Splines/test/walk.sli");assert_eq!(s.mirror,side==1);
                assert!((s.tex_offset-offset).abs()<1e-8);assert_eq!(s.map_chain_offset,Some(s.tex_offset));
                let start=point(*tile,s,0.0,0.0);if let Some(p)=last {assert!(start.distance(p)<1e-7);}
                last=Some(point(*tile,s,s.length,0.0));offset+=s.length;
            }
            assert!((offset-7.25-plan.actual).abs()<1e-8);
            assert!(last.unwrap().distance(plan.handles[1])<1e-7);
        }}}
        assert_eq!(def,original);
    }
    #[test] fn connected_roads_and_both_side_chains(){
        let r=road(50.0);let c=curve(&r);let p=c.point_at(r.spline.length);
        let mut second=r.clone();second.key.1=2;second.spline.pos=p.to_array();second.spline.heading=c.heading_at(r.spline.length);
        let plan=build(&[r,second],&profile(0.0,2.0),"walk.sli",&Settings{sides:3,start:70.0,length:30.0,..Default::default()},3.0).unwrap();
        assert_eq!(plan.pieces.iter().filter(|(_,s)|s.next_id==-1).count(),2);
        assert_eq!(plan.pieces.iter().filter(|(_,s)|s.tex_offset==3.0).count(),2);
        assert_eq!(plan.total,160.0);
    }
    #[test] fn grade_and_skew_follow_rendered_edge(){
        let mut r=road(0.0);r.spline.grad_start=2.0;r.spline.grad_end=4.0;r.spline.delta_h=Some(2.4);r.spline.is_h=true;r.spline.skew_start=0.2;r.spline.skew_end=-0.1;
        let plan=build(&[r.clone()],&profile(0.0,2.0),"walk.sli",&Settings{start:4.0,length:8.0,..Default::default()},0.0).unwrap();
        for ((tile,s),i) in plan.pieces.iter().zip(0..) {
            let start=point(*tile,s,0.0,0.0);let expected=frame(&r,4.0+i as f64*0.5,2,0.0,0.0).unwrap().0;
            assert!(start.distance(expected)<1e-6);assert!(s.profile_transitions.iter().all(Option::is_some));
        }
    }
    #[test] fn range_and_tight_curve_errors_do_not_produce_partial_plans(){
        let d=profile(0.0,2.0);let mut s=Settings::default();s.start=70.0;
        assert!(build(&[road(0.0)],&d,"walk.sli",&s,0.0).is_err());
        s.start=0.0;assert!(build(&[road(5.0)],&d,"walk.sli",&s,0.0).is_err());
        s.length=f64::NAN;assert!(build(&[road(0.0)],&d,"walk.sli",&s,0.0).is_err());
    }
    #[test] fn dragging_start_preserves_end_and_dragging_end_clamps(){
        let mut w=Window::new(None);w.preview.total=80.0;w.settings.start=10.0;w.settings.length=30.0;
        w.drag=Some(true);w.drag_to(25.0);assert_eq!(w.settings.start,25.0);assert_eq!(w.settings.length,15.0);
        w.drag_to(70.0);assert_eq!(w.settings.start,39.5);assert_eq!(w.settings.length,0.5);
        w.drag=Some(false);w.drag_to(100.0);assert_eq!(w.settings.length,40.5);
        w.drag_to(0.0);assert_eq!(w.settings.length,0.5);
    }
    #[test] fn station_projection_uses_route_direction(){
        let mut r=road(60.0);let p=curve(&r).point_at(27.0);
        assert!((project_station(&[r.clone()],p).unwrap()-27.0).abs()<0.01);
        r.backwards=true;assert!((project_station(&[r],p).unwrap()-53.0).abs()<0.01);
    }
}
