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
    pub bus_only:bool,
    pub width:f64,
}
impl Default for Connector {
    fn default()->Self {Self {source:None,destination:None,picking:Some(true),reach:[10.0;2],bus_only:false,width:3.0}}
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
            return Err("Mehrere Anschlüsse liegen übereinander. Kamera näher heran oder seitlich bewegen.".into());
        }
        Ok(Some(best))
    }
    pub fn accepts(&self,t:&Target)->bool {
        self.picking.is_some_and(|source|t.start!=source)
    }
    pub fn choose(&mut self,t:Target)->Result<(),String> {
        if !self.accepts(&t){return Err("Start braucht ein Fahrwegende, Ziel einen Fahrweganfang".into());}
        if self.picking==Some(true){self.source=Some(t);self.picking=Some(false);}else{self.destination=Some(t);self.picking=None;}
        if let (Some(a),Some(b))=(self.source,self.destination) {
            self.reach=[(a.point.distance(b.point)*0.4).clamp(1.0,40.0);2];
        }
        Ok(())
    }
    pub fn controls(&self)->Option<[DVec3;2]> {
        let a=self.source?;let b=self.destination?;
        Some([a.point+a.direction.normalize_or_zero()*self.reach[0],b.point-b.direction.normalize_or_zero()*self.reach[1]])
    }
    pub fn move_handle(&mut self,index:usize,point:DVec3){
        let Some(t)=(if index==0{self.source}else{self.destination})else{return;};
        let along=(point-t.point).dot(t.direction.normalize_or_zero())*if index==0{1.0}else{-1.0};
        self.reach[index.min(1)]=along.clamp(1.0,60.0);
    }
    pub fn status(&self)->String {
        match self.picking {
            Some(true)=>"1. Gelben Anschluss am ENDE der ankommenden Fahrspur anklicken".into(),
            Some(false)=>"2. Lila Anschluss am ANFANG der abgehenden Fahrspur anklicken".into(),
            None=>"3. Grüne Bogenpunkte ziehen; Richtung bleibt am Anschluss erhalten. Dann Verbindung hinzufügen.".into(),
        }
    }
    pub fn build(&self)->Result<Plan,String> {
        let (Some(a),Some(b))=(self.source,self.destination)else{return Ok(Plan {message:self.status(),..Default::default()});};
        if a.start || !b.start || (a.key==b.key&&a.reversed==b.reversed) {return Err("Zwei verschiedene Fahrwege in Fahrtrichtung auswählen".into());}
        if !self.width.is_finite()||!(1.5..=5.0).contains(&self.width)||!a.point.is_finite()||!b.point.is_finite()||!a.direction.is_finite()||!b.direction.is_finite()
            || a.direction.length()<0.9||b.direction.length()<0.9||!self.reach.iter().all(|r|r.is_finite()&&(1.0..=60.0).contains(r)) {
            return Err("Ungültige Anschlussgeometrie".into());
        }
        let delta=b.point-a.point;let length=delta.truncate().length();
        if !(1.0..=200.0).contains(&length) || delta.z.abs()>20.0 {return Err("Anschlüsse müssen 1 bis 200 m auseinander liegen; Höhenunterschied höchstens 20 m".into());}
        let [p1,p2]=self.controls().unwrap();
        let heading=delta.x.atan2(delta.y).to_degrees();let forward=SplineCurve::dir(heading).extend(0.0);
        let right=DVec3::new(forward.y,-forward.x,0.0);
        let tile_size=omsi_map::tile_size();
        let tile=((a.point.x/tile_size).floor() as i32,(a.point.y/tile_size).floor() as i32);
        let origin=DVec3::new(tile.0 as f64*tile_size,tile.1 as f64*tile_size,0.0);
        let mut s=Settings {connected:false,two_way:false,width:self.width,height:0.0,offset:0.0,forward_bus:self.bus_only,..Default::default()};
        let point=|t:f64| {let u=1.0-t;a.point*u*u*u+p1*3.0*u*u*t+p2*3.0*u*t*t+b.point*t*t*t};
        // Uniform nodes plus short tangent stubs ensure the saved lane joins in travel direction.
        for i in 0..=32 {
            let t=i as f64/32.0;let d=point(t)-(a.point+forward*length*t);
            if d.dot(right).abs().max(d.dot(forward).abs()).max(d.z.abs())>49.999{return Err("Bogen zu weit von der Verbindung entfernt".into());}
            s.nodes.push((0,[t,d.dot(right),d.dot(forward),d.z]));
        }
        for (outgoing,t) in [(false,a),(true,b)] {s.connections.push((0,outgoing,[t.key.tile.0 as i64,t.key.tile.1 as i64,t.key.id,t.key.path as i64,i64::from(t.reversed)]));}
        let mut piece=MapSpline {id:0,pos:(a.point-origin).to_array(),heading,length,next_id:-1,rules:crate::traffic_editor::rules(&s),..Default::default()};
        // Distinct from road-following paths: it may branch from a lane with a fixed next link.
        piece.rules.push(MapRule {path_index:0,kind:"editor_additional_connection".into(),value:1.0,..Default::default()});
        let body=crate::traffic_editor::definition(&s,false);
        let identity=format!("{body}\n{:?}\n{heading:.17}\n{length:.17}\n{}",piece.pos,self.bus_only);
        piece.file=format!("{PREFIX}{:x}.sli",Sha256::digest(identity.as_bytes()));
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
        plan.handles=vec![(0,p1),(1,p2)];
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
}
