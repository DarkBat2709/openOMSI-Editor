//! Additional directed links between existing lane endpoints. Never replaces a road.
use crate::traffic_editor::{Plan, Settings, Target};
use glam::{DVec2,DVec3};
use omsi_geometry::SplineCurve;
use omsi_map::{MapRule,MapSpline};
use sha2::{Digest,Sha256};

pub const PREFIX:&str="Splines/openOMSI_Editor/AI/connector_";
#[derive(Clone)]
pub struct Connector {
    pub source:Option<Target>,
    pub destination:Option<Target>,
    /// Some(true): choose source lane END. Some(false): choose target lane START.
    pub picking:Option<bool>,
    pub reach:[f64;2],
    pub bend:Option<[DVec3;3]>,
    pub free:Option<omsi_geometry::free_path::FreePath>,
    pub unbound:bool,
    pub bus_only:bool,
    pub width:f64,
}
impl Default for Connector {
    fn default()->Self {Self {source:None,destination:None,picking:Some(true),reach:[10.0;2],bend:None,free:None,unbound:false,bus_only:false,width:3.0}}
}
impl Connector {
    pub fn pick(&self,targets:&[Target],origin:DVec3,ray:DVec3)->Result<Option<Target>,String> {
        let mut hits:Vec<_>=targets.iter().filter(|t|self.accepts(t)).filter_map(|t|{
            let delta=t.point+DVec3::Z*0.4-origin;let along=delta.dot(ray);
            let distance=(delta-ray*along).length();
            (along>0.0&&along<1000.0&&distance<(along*0.006).clamp(0.35,1.5)).then_some((distance,*t))
        }).collect();
        hits.sort_by(|a,b|a.0.total_cmp(&b.0));
        let Some((distance,best))=hits.first().copied()else{return Ok(None);};
        if hits.iter().skip(1).any(|(d,t)|(*d-distance).abs()<0.1&&(t.key!=best.key||t.reversed!=best.reversed)) {
            return Err("Several endpoints overlap. Move the camera closer or to the side.".into());
        }
        Ok(Some(best))
    }
    pub fn accepts(&self,t:&Target)->bool {
        self.picking.is_some_and(|source|t.start!=source)
    }
    pub fn choose(&mut self,t:Target)->Result<(),String> {
        if !self.accepts(&t){return Err("Start requires a path end; destination requires a path start".into());}
        if self.picking==Some(true){self.source=Some(t);self.picking=Some(false);}else{self.destination=Some(t);self.picking=None;}
        if let (Some(a),Some(b))=(self.source,self.destination) {
            self.reach=[(a.point.distance(b.point)*0.4).clamp(1.0,40.0);2];self.bend=None;
        }
        Ok(())
    }
    pub fn controls(&self)->Option<[DVec3;3]> {
        let a=self.source?;let b=self.destination?;
        if let Some(p)=self.bend{return Some(p);}
        Some(omsi_geometry::free_path::elevate(a.point,a.point+a.direction.normalize_or_zero()*self.reach[0],b.point-b.direction.normalize_or_zero()*self.reach[1],b.point))
    }
    pub fn move_handle(&mut self,index:usize,point:DVec3){
        if let Some(f)=self.free.as_mut(){f.move_handle(index,point);return;}
        let Some(mut controls)=self.controls()else{return;};
        if !point.is_finite(){return;}
        controls[index.min(2)]=point;self.bend=Some(controls);
    }
    pub fn complete(&self)->bool {self.free.as_ref().map_or(self.source.is_some()&&self.destination.is_some()&&self.picking.is_none(),|f|f.points.len()>1)}
    pub fn handles(&self)->Vec<(usize,DVec3)>{self.free.as_ref().map_or_else(||self.controls().map(|c|c.into_iter().enumerate().collect()).unwrap_or_default(),|f|f.handles())}
    pub fn status(&self)->String {
        if self.free.is_some(){return "Click to add sections. Drag endpoints and three form handles. Add when ready.".into();}
        match self.picking {
            Some(true)=>"1. Click the yellow endpoint at the END of the incoming lane".into(),
            Some(false)=>"2. Click the purple endpoint at the START of the outgoing lane".into(),
            None=>"3. Drag the green curve handles freely. The connection follows the loaded road surface. Then add the connection.".into(),
        }
    }
    #[cfg(test)]
    pub fn build(&self)->Result<Plan,String> {self.build_projected(None)}
    pub fn build_on_road(&self,height:&dyn Fn(DVec3)->Option<f64>)->Result<Plan,String> {self.build_projected(Some(height))}
    fn build_projected(&self,height:Option<&dyn Fn(DVec3)->Option<f64>>)->Result<Plan,String> {
        if let Some(f)=&self.free {
            let mut plan=Plan::default();
            for i in 0..f.points.len().saturating_sub(1){let mut c=self.clone();c.free=None;c.unbound=true;c.picking=None;c.bend=Some(f.controls(i));
                let target=|j,start|Target {point:f.points[j],direction:f.direction(j),start,key:omsi_sim::traffic::LaneKey {tile:(0,0),id:j as i64,path:0},reversed:false};
                c.source=Some(target(i,false));c.destination=Some(target(i+1,true));let p=c.build_projected(height)?;
                plan.markers.extend(p.markers);plan.endpoints.extend(p.endpoints);plan.definitions.extend(p.definitions);plan.pieces.extend(p.pieces);
            }plan.handles=f.handles();plan.message=self.status();return Ok(plan);
        }
        let (Some(a),Some(b))=(self.source,self.destination)else{return Ok(Plan {message:self.status(),..Default::default()});};
        if a.start || !b.start || (a.key==b.key&&a.reversed==b.reversed) {return Err("Select two different paths in the direction of travel".into());}
        if !self.width.is_finite()||!(1.5..=5.0).contains(&self.width)||!a.point.is_finite()||!b.point.is_finite()||!a.direction.is_finite()||!b.direction.is_finite()
            || a.direction.length()<0.9||b.direction.length()<0.9||!self.reach.iter().all(|r|r.is_finite()&&(1.0..=60.0).contains(r)) {
            return Err("Invalid connection geometry".into());
        }
        let delta=b.point-a.point;let length=delta.truncate().length();
        if !(1.0..=200.0).contains(&length) || delta.z.abs()>20.0 {return Err("Endpoints must be 1 to 200 m apart; height difference must not exceed 20 m".into());}
        let controls=self.controls().unwrap();
        if controls.iter().any(|p|!p.is_finite()){return Err("Invalid curve handles".into());}
        let heading=delta.x.atan2(delta.y).to_degrees();let forward=SplineCurve::dir(heading).extend(0.0);
        let right=DVec3::new(forward.y,-forward.x,0.0);
        let tile_size=omsi_map::tile_size();
        let tile=((a.point.x/tile_size).floor() as i32,(a.point.y/tile_size).floor() as i32);
        let origin=DVec3::new(tile.0 as f64*tile_size,tile.1 as f64*tile_size,0.0);
        let mut s=Settings {connected:false,two_way:false,width:self.width,height:0.0,offset:0.0,forward_bus:self.bus_only,..Default::default()};
        let point=|t:f64|omsi_geometry::free_path::point(a.point,controls,b.point,t);
        // Uniform nodes plus short tangent stubs ensure the saved lane joins in travel direction.
        let steps=(length/0.5).ceil().clamp(32.0,127.0) as usize;
        for i in 0..=steps {
            let t=i as f64/steps as f64;let mut p=point(t);
            if i>0&&i<steps {if let Some(height)=height {p.z=height(p).ok_or("No loaded road surface under the curve. Drag the curve onto the road or load the area.")?;}}
            let d=p-(a.point+forward*length*t);
            if d.dot(right).abs().max(d.dot(forward).abs()).max(d.z.abs())>49.999{return Err("Curve is too far from the connection".into());}
            s.nodes.push((0,[t,d.dot(right),d.dot(forward),d.z]));
        }
        if !self.unbound {for (outgoing,t) in [(false,a),(true,b)] {s.connections.push((0,outgoing,[t.key.tile.0 as i64,t.key.tile.1 as i64,t.key.id,t.key.path as i64,i64::from(t.reversed)]));}}
        let mut piece=MapSpline {id:0,pos:(a.point-origin).to_array(),heading,length,next_id:-1,rules:crate::traffic_editor::rules(&s),..Default::default()};
        // Distinct from road-following paths: it may branch from a lane with a fixed next link.
        piece.rules.push(MapRule {path_index:0,kind:"editor_additional_connection".into(),value:1.0,..Default::default()});
        let body=crate::traffic_editor::definition(&s,false);
        let identity=format!("{body}\n{:?}\n{heading:.17}\n{length:.17}\n{}",piece.pos,self.bus_only);
        piece.file=format!("{PREFIX}{}.sli",Sha256::digest(identity.as_bytes()).iter().map(|b|format!("{b:02x}")).collect::<String>());
        let curve=SplineCurve::from_map(&piece,DVec2::new(origin.x,origin.y));
        let sample=|station:f64|omsi_geometry::editor_path_point(&curve,&s.nodes,0,station,0.0,0.0);
        let mut plan=Plan::default();
        let count=(length/0.5).ceil().clamp(4.0,400.0) as usize;
        for i in 0..=count {plan.markers.push((sample(length*i as f64/count as f64),self.bus_only));}
        for i in 0..=(length/6.0).floor() as usize {
            let station=(i as f64*6.0+length.min(2.0)*0.5).min(length);
            let p=sample(station);let d=(sample((station+0.1).min(length))-sample((station-0.1).max(0.0))).normalize_or_zero();
            let side=DVec3::new(d.y,-d.x,0.0);
            for t in [0.0,0.25,0.5,0.75,1.0] {for sign in [-1.0,1.0] {plan.markers.push((p-d*t+side*t*0.6*sign,self.bus_only));}}
        }
        plan.handles=controls.into_iter().enumerate().collect();
        if let Some(height)=height {for (_,p) in &mut plan.handles {if let Some(z)=height(*p){p.z=z;}}}
        plan.endpoints=vec![(a.point,a.direction,true),(b.point,b.direction,false)];
        plan.definitions.push((piece.file.clone(),body));plan.pieces.push((tile,piece));plan.message=self.status();
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omsi_sim::traffic::LaneKey;
    fn target(id:i64,p:DVec3,d:DVec3,start:bool)->Target {Target {point:p,direction:d,start,key:LaneKey {tile:(0,0),id,path:0},reversed:false}}
    #[test]
    #[ignore="requires graphics adapter; writes free path UI preview"]
    fn render_free_path_ui(){
        let instance=crate::graphics_instance();let mut renderer=pollster::block_on(omsi_render::Renderer::new_with(&instance,None,Some(wgpu::TextureFormat::Rgba8UnormSrgb),omsi_render::RenderOptions{msaa:1,ssao:false,fxaa:false,shadow_size:1024,..Default::default()})).unwrap();
        let camera=omsi_render::Camera{position:DVec3::new(0.0,0.0,10.0),yaw:0.0,pitch:0.0,roll:0.0,fov_deg:60.0,near:0.1,far:1000.0};
        for lang in ["DEU","ENG"]{crate::ui_language(lang);let mut w=crate::traffic_editor::Window::new(((0,0),0));let mut c=Connector::default();c.picking=None;c.free=Some(Default::default());c.free.as_mut().unwrap().add(DVec3::ZERO).unwrap();c.free.as_mut().unwrap().add(DVec3::Y*20.0).unwrap();w.preview=c.build().unwrap();w.connector=Some(c);
            let mut scene=renderer.new_scene();let mut ui=crate::ui::Ui::new().unwrap();ui.draw_traffic_window(&renderer,&mut scene,Some(&mut w),true,1.0,[0.0,0.0,1280.0,800.0],(-1.0,-1.0));
            for command in [crate::traffic_editor::Command::ConnectorFree,crate::traffic_editor::Command::ConnectorUndo,crate::traffic_editor::Command::Apply]{let r=w.rects.iter().find(|(_,c)|std::mem::discriminant(c)==std::mem::discriminant(&command)).unwrap().0;let cursor=((r[0]+r[2])/2.0,(r[1]+r[3])/2.0);assert!(!ui.editor_window_drag_press(cursor));assert_eq!(std::mem::discriminant(&w.hit(cursor).unwrap()),std::mem::discriminant(&command));}
            let rgba=renderer.render_to_image(&mut scene,1280,800,&camera,&omsi_render::Lighting{shadows:false,..Default::default()}).unwrap();image::save_buffer(format!("/tmp/free-ai-{lang}.png"),&rgba,1280,800,image::ColorType::Rgba8).unwrap();
        }
    }
    #[test]
    fn free_chain_saves_three_handles_per_section_and_continuous_tile_crossings(){
        let mut c=Connector::default();c.picking=None;c.free=Some(Default::default());
        for point in [DVec3::new(290.0,10.0,0.0),DVec3::new(310.0,10.0,0.0),DVec3::new(330.0,20.0,0.0)] {c.free.as_mut().unwrap().add(point).unwrap();}
        let before=c.build().unwrap();assert_eq!(before.handles.len(),9);assert_eq!(before.pieces.len(),2);assert!(before.replacements.is_empty());
        c.move_handle(1,DVec3::new(300.0,16.0,0.0));let after=c.build().unwrap();assert_ne!(before.definitions[0].0,after.definitions[0].0);
        let mut lanes=Vec::new();
        for ((tile,piece),(name,text)) in after.pieces.iter().zip(&after.definitions){
            let def=omsi_scenery::Spline::parse(&omsi_cfg::CfgFile::from_str(name,text));assert!(def.editor_path_connections.is_empty());assert!(def.profiles.is_empty());
            let curve=SplineCurve::from_map(piece,DVec2::new(tile.0 as f64*omsi_map::tile_size(),tile.1 as f64*omsi_map::tile_size()));
            let mut piece=piece.clone();piece.id=lanes.len() as i64+1;lanes.extend(omsi_scene::lanes::spline_lanes(&def,&piece,&curve,*tile));
        }
        let mut net=omsi_sim::traffic::Network {lanes,..Default::default()};net.link(1.5);assert!(net.lanes[0].next.contains(&1));
        assert!(net.lanes[0].end().distance(net.lanes[1].start())<0.001);
        c.free.as_mut().unwrap().undo();assert_eq!(c.build().unwrap().pieces.len(),1);
    }
    #[test]
    fn connector_adds_invisible_curve_with_persistent_directed_links() {
        let mut c=Connector::default();
        c.choose(target(1,DVec3::new(10.0,10.0,0.0),DVec3::X,false)).unwrap();
        c.choose(target(2,DVec3::new(30.0,30.0,1.0),DVec3::Y,true)).unwrap();
        let p=c.build().unwrap();assert_eq!(p.pieces.len(),1);assert!(p.replacements.is_empty());
        assert!(crate::traffic_editor::source_of_generated_path(&p.pieces[0].1.file).is_none());
        let def=omsi_scenery::Spline::parse(&omsi_cfg::CfgFile::from_str(&p.definitions[0].0,&p.definitions[0].1));
        assert!(def.profiles.is_empty());assert_eq!(def.paths.len(),1);assert_eq!(def.editor_path_connections.len(),2);
        let curve=SplineCurve::from_map(&p.pieces[0].1,DVec2::ZERO);
        let f=|s|omsi_geometry::editor_path_point(&curve,&def.editor_path_nodes,0,s,0.0,0.0);
        assert!(f(0.0).distance(c.source.unwrap().point)<1e-6);
        assert!(f(curve.length).distance(c.destination.unwrap().point)<1e-6);
        assert!((f(0.1)-f(0.0)).normalize().dot(DVec3::X)>0.98);
        assert!((f(curve.length)-f(curve.length-0.1)).normalize().dot(DVec3::Y)>0.98);
        let name=p.pieces[0].1.file.clone();c.move_handle(0,DVec3::new(25.0,50.0,0.0));
        assert_ne!(name,c.build().unwrap().pieces[0].1.file);
        c.bus_only=true;let p=c.build().unwrap();assert!(p.pieces[0].1.rules.iter().any(|r|r.kind=="editor_bus_only"));
    }
    #[test]
    fn connector_rejects_wrong_direction_missing_and_too_distant_endpoints() {
        let mut c=Connector::default();assert!(c.build().unwrap().pieces.is_empty());
        assert!(c.choose(target(1,DVec3::ZERO,DVec3::Y,true)).is_err());
        c.choose(target(1,DVec3::ZERO,DVec3::Y,false)).unwrap();
        c.choose(target(2,DVec3::Y*300.0,DVec3::Y,true)).unwrap();assert!(c.build().is_err());
    }
    #[test]
    fn free_controls_and_road_height_are_saved_in_generated_nodes() {
        let mut c=Connector::default();
        c.choose(target(1,DVec3::ZERO,DVec3::Y,false)).unwrap();
        c.choose(target(2,DVec3::new(0.0,30.0,0.0),DVec3::Y,true)).unwrap();
        let control=DVec3::new(8.0,9.0,0.0);c.move_handle(0,control);
        assert_eq!(c.controls().unwrap()[0],control);
        let height=|p:DVec3|Some((p.y/30.0*std::f64::consts::PI).sin()*3.0);
        let plan=c.build_on_road(&height).unwrap();
        let def=omsi_scenery::Spline::parse(&omsi_cfg::CfgFile::from_str(&plan.definitions[0].0,&plan.definitions[0].1));
        let curve=SplineCurve::from_map(&plan.pieces[0].1,DVec2::ZERO);
        for i in 0..=60 {
            let p=omsi_geometry::editor_path_point(&curve,&def.editor_path_nodes,0,curve.length*i as f64/60.0,0.0,0.0);
            assert!((p.z-height(p).unwrap()).abs()<0.004,"{p:?}");
        }
        assert!(c.build_on_road(&|_|None).is_err());
    }

}
