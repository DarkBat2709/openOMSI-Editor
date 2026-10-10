//! Additional, invisible driving paths following existing surface splines.
//! Source geometry is never edited; map changes use the spline editor's undo/save.
use crate::{scene::World, spline_editor::Key};
use glam::{DVec2, DVec3};
use omsi_geometry::SplineCurve;
use omsi_map::{MapRule, MapSpline};
use std::path::Path;

/// Immutable generated filenames retain the original road identity across map reloads.
pub fn source_of_generated_path(file:&str)->Option<Key> {
    let file=file.replace('\\',"/");
    let name=file.strip_prefix("Splines/openOMSI_Editor/AI/ai_")?;
    let mut parts=name.split('_');
    Some(((parts.next()?.parse().ok()?,parts.next()?.parse().ok()?),parts.next()?.parse().ok()?))
}
type OverlayResult = (u64, DVec3, Vec<omsi_sim::traffic::Lane>);
#[derive(Default)]
pub struct Overlay {
    pub lanes:Vec<omsi_sim::traffic::Lane>,
    center:Option<DVec3>,
    generation:u64,
    pending:Option<std::sync::mpsc::Receiver<OverlayResult>>,
    world:Option<std::sync::Weak<World>>,
}
impl Overlay {
    pub fn invalidate(&mut self){
        self.generation=self.generation.wrapping_add(1);
        self.center=None;
        self.lanes.clear();
    }
    #[cfg(test)]
    pub(crate) fn wait_for_test(&mut self, world:&std::sync::Arc<World>, center:DVec3) {
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        loop {
            self.refresh(world,center);
            if self.pending.is_none() {break;}
            assert!(std::time::Instant::now()<deadline,"overlay worker timed out");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    fn accept(&mut self, result:OverlayResult) {
        let (generation,center,lanes)=result;
        if generation==self.generation {self.lanes=lanes;self.center=Some(center);}
    }
    pub fn refresh(&mut self,world:&std::sync::Arc<World>,center:DVec3){
        let same_world=self.world.as_ref().and_then(std::sync::Weak::upgrade)
            .is_some_and(|old|std::sync::Arc::ptr_eq(&old,world));
        if !same_world {
            self.invalidate();
            self.world=Some(std::sync::Arc::downgrade(world));
        }
        if let Some(rx)=self.pending.as_ref() {
            match rx.try_recv() {
                Ok(result)=>{self.pending=None;self.accept(result);}
                Err(std::sync::mpsc::TryRecvError::Empty)=>return,
                Err(std::sync::mpsc::TryRecvError::Disconnected)=>{
                    self.pending=None;
                    self.center=Some(center);
                    log::warn!("AI path overlay worker stopped; toggle path display to retry");
                    return;
                }
            }
        }
        if self.center.is_some_and(|old|old.truncate().distance(center.truncate())<=100.0){return;}
        let (tx,rx)=std::sync::mpsc::channel();
        let world=world.clone();let generation=self.generation;
        match std::thread::Builder::new().name("editor path overlay".into()).spawn(move || {
            crate::threads::lower_thread_priority();
            let lanes=crate::roadside_objects::traffic_overlay_lanes(&world,center);
            let _=tx.send((generation,center,lanes));
        }) {
            Ok(_)=>self.pending=Some(rx),
            Err(error)=>{
                self.center=Some(center);
                log::warn!("AI path overlay worker could not start: {error}; toggle path display to retry");
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub nodes:Vec<(usize,[f64;4])>,
    pub connections:Vec<(usize,bool,[i64;5])>,
    pub connected: bool,
    pub two_way: bool,
    pub parallel: bool,
    pub reverse: bool,
    pub width: f64,
    pub offset: f64,
    pub height: f64,
    pub forward_bus: bool,
    pub backward_bus: bool,
}
impl Default for Settings {
    fn default() -> Self { Self { connections:Vec::new(),nodes:Vec::new(),connected:true, two_way:true,parallel:false, reverse:false, width:3.0,
        offset:0.0, height:0.1, forward_bus:false, backward_bus:false } }
}
impl Settings {
    pub fn toggle_two_way(&mut self) {
        // Keep lane 1 on its current carriageway when adding/removing the opposite lane.
        if self.two_way&&self.parallel {self.parallel=false;self.connections.retain(|(i,_,_)|*i==0);return;}
        self.offset=(self.offset+if self.two_way {self.width*0.5}else{-self.width*0.5}).clamp(-20.0,20.0);
        self.two_way=!self.two_way;self.parallel=self.two_way;
        if !self.two_way {self.nodes.retain(|(i,_)|*i==0);self.connections.retain(|(i,_,_)|*i==0);}
    }
}
#[derive(Default)]
pub struct Plan {
    pub pieces: Vec<((i32,i32),MapSpline)>,
    pub definitions: Vec<(String,String)>,
    pub markers: Vec<(DVec3,bool)>,
    pub endpoints: Vec<(DVec3,DVec3,bool)>, // point, driving tangent, start?
    pub message: String,
    pub replacements:Vec<(Key,MapSpline,MapSpline)>,
    pub handles:Vec<(usize,DVec3)>,
}
fn rule(index: usize, kind: &str) -> MapRule {
    MapRule { path_index:index as i32, kind:kind.into(), value:1.0, ..Default::default() }
}
pub fn rules(s:&Settings) -> Vec<MapRule> {
    let mut out=Vec::new();
    for i in 0..if s.two_way {2} else {1} {
        let bus=if i==0 {s.forward_bus} else {s.backward_bus};
        out.push(rule(i,"bus"));
        out.push(rule(i,"editor_surface_path"));
        if bus {out.push(rule(i,"no_cars"));out.push(rule(i,"editor_bus_only"));}
        else {out.push(rule(i,"trucks"));}
    }
    out
}
/// (physical lateral offset, direction in the original spline, access).
pub fn lanes(s:&Settings, backwards:bool) -> Vec<(f64,i32,bool)> {
    let sign=if backwards {-1.0} else {1.0};
    let reverse=s.reverse ^ backwards;
    let mut v=vec![((s.offset+if s.two_way {s.width*0.5} else {0.0})*sign,
        if reverse {1} else {0},s.forward_bus)];
    if s.two_way {v.push(((s.offset-s.width*0.5)*sign,if reverse ^ s.parallel {0} else {1},s.backward_bus));}
    v
}
pub fn definition(s:&Settings, backwards:bool) -> String {
    let mut out=String::from("[friendlyname]\nEditor AI paths\n\n[length]\n10\n\n");
    for (x,dir,_) in lanes(s,backwards) {
        out+=&format!("[path]\n0\n{x:.6}\n{:.6}\n{:.6}\n{dir}\n\n",s.height,s.width);
    }
    for (index,outgoing,v) in &s.connections {out+=&format!("[editor_path_connection]\n{index}\n{}\n{}\n{}\n{}\n{}\n{}\n\n",if *outgoing{1}else{0},v[0],v[1],v[2],v[3],v[4]);}
    for (index,n) in &s.nodes {out+=&format!("[editor_path_node]\n{index}\n{:.12}\n{:.12}\n{:.12}\n{:.12}\n\n",n[0],n[1],n[2],n[3]);}
    out
}
fn driving_tangent(c:&SplineCurve,nodes:&[(usize,[f64;4])],pi:usize,a:f64,x:f64,z:f64,dir:i32)->DVec3 {
    let d=omsi_geometry::editor_path_point(c,nodes,pi,(a+0.05).min(c.length),x,z)-omsi_geometry::editor_path_point(c,nodes,pi,(a-0.05).max(0.0),x,z);
    d.truncate().normalize_or_zero().extend(0.0)*if dir==1 {-1.0}else{1.0}
}
pub fn build(roads:&[(Key,MapSpline,bool)],s:&Settings) -> Result<Plan,String> {
    if roads.is_empty() || roads.len()>500 {return Err("Bitte einen Spline oder eine Kette bis 500 Abschnitte wählen".into());}
    if ![s.width,s.offset,s.height].iter().all(|v|v.is_finite()) || !(1.5..=5.0).contains(&s.width)
        || s.offset.abs()>20.0 || s.height.abs()>5.0 {return Err("Ungültige Spurbreite, Verschiebung oder Höhe".into());}
    if s.nodes.len()>128 || s.nodes.iter().any(|(i,n)|*i>=if s.two_way{2}else{1} || !n.iter().all(|v|v.is_finite()) || !(0.0..=1.0).contains(&n[0]) || n[1..].iter().any(|v|v.abs()>50.0)) {
        return Err("Ungültige Freiformpunkte (maximal 50 m Verschiebung)".into());
    }
    if !s.nodes.is_empty()&&(roads.len()!=1||roads[0].2){return Err("Freiformpunkte bitte am einzelnen Abschnitt bearbeiten".into());}
    let mut plan=Plan::default();
    let mut last:Option<DVec3>=None;
    for (key,original,backwards) in roads {
        if original.deleted || ![original.length,original.heading,original.radius,original.grad_start,original.grad_end,original.cant_start,original.cant_end,original.skew_start,original.skew_end,original.delta_h.unwrap_or(0.0)].iter().chain(original.pos.iter()).all(|n|n.is_finite()) || original.length<0.5 || original.length>10000.0 {
            return Err("Ungültige Länge des Referenzsplines".into());
        }
        if original.profile_transitions.iter().flatten().any(|t|!t.valid()) {
            return Err(format!("Spline {}: ungültige Profilkorrektur",key.1));
        }
        // The new surface-path rule makes preview and saved AI lanes use mesh geometry,
        // including the source definition's cant width and longitudinal skew.
        let mut piece=original.clone();piece.id=0;piece.prev_id=0;piece.next_id=-1;
        piece.mirror=false;piece.terrain_align=None;piece.terrain_align_flag=false;piece.rules=rules(s);
        let half_width=omsi_geometry::half_cant_width_of(&original.file);
        let body=format!("{}[halfcantwidth]\n{half_width:.17}\n",definition(s,*backwards));
        let name=format!("Splines/openOMSI_Editor/AI/ai_{}_{}_{}_{}_{}_{}_{}_{}.sli",
            key.0.0,key.0.1,key.1,if *backwards {1}else{0},if s.two_way {2}else{1},if s.reverse {1}else{0},
            (s.width*1000.0).round() as i64,format!("{}_{}",(s.offset*1000.0).round() as i64,(s.height*1000.0).round() as i64));
        let name=name.replace(".sli",&format!("_v14_{:x}.sli",half_width.to_bits()));
        let name=if s.nodes.is_empty()&&s.connections.is_empty()&&!s.parallel{name}else{use sha2::{Digest,Sha256};name.replace(".sli",&format!("_free_{:x}.sli",Sha256::digest(body.as_bytes())))};
        piece.file=name.clone();plan.definitions.push((name,body));
        let c=SplineCurve {half_cant_width:half_width,..SplineCurve::from_map(&piece,DVec2::new(key.0.0 as f64,key.0.1 as f64)*omsi_map::tile_size())};
        let start=c.point_at(if *backwards {piece.length}else{0.0});
        if last.is_some_and(|p|p.distance(start)>0.25) {return Err("Lücke in der Splinekette: zuerst die Referenzsplines verbinden".into());}
        last=Some(c.point_at(if *backwards {0.0}else{piece.length}));
        for (pi,(x,dir,bus)) in lanes(s,*backwards).into_iter().enumerate() {
            if piece.radius.abs()>1e-6 && (1.0-x/piece.radius)<0.1 {return Err("Kurve für diesen Spurversatz zu eng".into());}
            let n=(piece.length/if s.nodes.is_empty(){2.0}else{0.5}).ceil().clamp(1.0,20000.0) as usize;
            let point=|a:f64|omsi_geometry::editor_path_point(&c,&s.nodes,pi,a,x,s.height);
            for (index,(_,node)) in s.nodes.iter().enumerate().filter(|(_,n)|n.0==pi) {plan.handles.push((index,point(node[0]*piece.length)));}
            for j in 0..=n {plan.markers.push((point(piece.length*j as f64/n as f64),bus));}
            for j in 0..=((piece.length/12.0).floor() as usize).min(1000) {
                let a=(j as f64*12.0+piece.length.min(4.0)*0.5).min(piece.length);
                let p=point(a);let d=driving_tangent(&c,&s.nodes,pi,a,x,s.height,dir);
                let right=DVec3::new(d.y,-d.x,0.0);
                for t in [0.0,0.25,0.5,0.75,1.0] {for side in [-1.0,1.0] {plan.markers.push((p-d*t+right*t*0.6*side,bus));}}
            }
            for at_start in [true,false] {
                let a=if at_start ^ (dir==1) {0.0}else{piece.length};
                plan.endpoints.push((point(a),driving_tangent(&c,&s.nodes,pi,a,x,s.height,dir),at_start));
            }
        }
        if plan.markers.len()>100000 {return Err("Vorschau zu groß: kürzere Kette wählen".into());}
        plan.pieces.push((key.0,piece));
    }
    if plan.markers.len()>100000 {return Err("Vorschau zu groß: kürzere Kette wählen".into());}
    Ok(plan)
}
/// Build only missing contiguous runs; existing paths retain their geometry and rules.
fn build_missing(roads:&[(Key,MapSpline,bool)],s:&Settings,occupied:&hashbrown::HashSet<Key>)->Result<Plan,String> {
    let mut result=Plan::default();
    for run in roads.split(|r|occupied.contains(&r.0)).filter(|run|!run.is_empty()) {
        let mut p=build(run,s)?;
        result.pieces.append(&mut p.pieces);result.definitions.append(&mut p.definitions);
        result.handles.append(&mut p.handles);result.markers.append(&mut p.markers);result.endpoints.append(&mut p.endpoints);
        if result.markers.len()>100000 {return Err("Vorschau zu groß: kürzere Kette wählen".into());}
    }
    if result.pieces.is_empty() {return Err("Alle ausgewählten Abschnitte haben bereits KI-Pfade. Vorhandene Pfade bleiben unverändert.".into());}
    Ok(result)
}
pub fn write_definitions(plan:&Plan,root:&Path,original:&Path)->Result<(),String> {
    use std::io::Write;
    // Assets are immutable. Never overwrite an existing definition used by a map.
    for (name,body) in &plan.definitions {
        let path=root.join(name);crate::editor::protect_output(&path,original)?;
        if path.exists() && std::fs::read(&path).map_err(|e|e.to_string())?!=body.as_bytes() {
            return Err(format!("Abweichende KI-Profildatei vorhanden: {}",path.display()));
        }
    }
    for (name,body) in &plan.definitions {
        let path=root.join(name);if path.exists(){continue;}
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;
        if let Err(e)=file.write_all(body.as_bytes()).and_then(|_|file.sync_all()) {drop(file);let _=std::fs::remove_file(path);return Err(e.to_string());}
    }
    omsi_cfg::content_changed();Ok(())
}
#[derive(Clone,Copy)]
pub enum Command { Close, Overlay, Free, Lane, ResetNode, NodeHeight(f64), Replace, Connected, TwoWay, Reverse, ForwardBus, BackwardBus, Width(f64),Offset(f64),Height(f64),Apply,Refresh }
#[derive(Clone,Copy)]
pub struct Target {pub point:DVec3,pub direction:DVec3,pub start:bool,pub key:omsi_sim::traffic::LaneKey,pub reversed:bool}
pub struct Window {pub drag_pending:bool,pub drag_events:usize,pub drag_queued_at:Option<std::time::Instant>,pub targets:Vec<Target>,pub free:bool,pub active_lane:usize,pub selected_node:Option<usize>,pub drag:Option<(usize,f64,DVec3)>,pub replace:bool,pub start:Key,pub settings:Settings,pub preview:Plan,pub error:Option<String>,pub rects:Vec<([f32;4],Command)>,pub rect:Option<[f32;4]>}
impl Window {
    pub fn new(start:Key)->Self {Self {drag_pending:false,drag_events:0,drag_queued_at:None,targets:Vec::new(),free:false,active_lane:0,selected_node:None,drag:None,replace:false,start,settings:Settings::default(),preview:Plan::default(),error:None,rects:Vec::new(),rect:None}}
    pub fn load_existing(&mut self,world:&World) {
        let Ok(roads)=crate::roadside_objects::traffic_route(world,self.start,false) else{return;};
        let key=roads[0].0;let found=crate::roadside_objects::generated_traffic(world,&[key]);
        let Some(existing)=found.get(&key).filter(|v|v.len()==1) else{return;};
        let (_,old)=&existing[0];let Some(ty)=world.spline_type(&old.file) else{return;};
        let paths=&ty.def.paths;
        if !(1..=2).contains(&paths.len())||paths.iter().any(|p|p.kind!=0||!(0..=1).contains(&p.direction)){return;}
        self.settings.nodes=ty.def.editor_path_nodes.clone();
        self.settings.connections=ty.def.editor_path_connections.clone();
        self.start=key;self.replace=true;self.settings.connected=false;
        self.settings.two_way=paths.len()==2;self.settings.parallel=paths.len()==2&&paths[0].direction==paths[1].direction;self.settings.reverse=paths[0].direction==1;
        self.settings.width=paths[0].width as f64;self.settings.height=paths[0].start[2] as f64;
        self.settings.offset=paths.iter().map(|p|p.start[0] as f64).sum::<f64>()/paths.len() as f64;
        self.settings.forward_bus=old.rules.iter().any(|r|r.path_index==0&&r.kind=="editor_bus_only"&&!r.kill);
        self.settings.backward_bus=old.rules.iter().any(|r|r.path_index==1&&r.kind=="editor_bus_only"&&!r.kill);
    }
    pub fn enable_free(&mut self) {
        self.free=!self.free;self.drag=None;self.settings.connected=false;
        if self.free {self.ensure_nodes();}
    }
    pub fn ensure_nodes(&mut self) {
        if !self.settings.two_way {self.active_lane=0;}
        if !self.settings.nodes.iter().any(|(i,_)|*i==self.active_lane) {
            for i in 0..=8 {self.settings.nodes.push((self.active_lane,[i as f64/8.0,0.0,0.0,0.0]));}
        }
        self.selected_node=None;
    }
    pub fn node_is_start(&self,index:usize)->Option<bool> {
        let (pi,n)=*self.settings.nodes.get(index)?;
        if n[0]!=0.0&&n[0]!=1.0{return None;}
        let (_,dir,_)=*lanes(&self.settings,false).get(pi)?;
        Some((n[0]==0.0)^(dir==1))
    }
    pub fn snap_node(&mut self,index:usize,target:Target)->bool {
        let Some(start)=self.node_is_start(index) else{return false;};
        if start==target.start{return false;}
        let Some((_,piece))=self.preview.pieces.first() else{return false;};let length=piece.length;
        let (pi,node)=self.settings.nodes[index];
        self.move_node(index,target.point);
        let Some((tile,piece))=self.preview.pieces.first() else{return false;};
        let def=omsi_scenery::Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,&self.preview.definitions[0].1));
        let c=SplineCurve::from_map(piece,DVec2::new(tile.0 as f64,tile.1 as f64)*omsi_map::tile_size()).with_sli(&def);
        let (x,_,_)=lanes(&self.settings,false)[pi];
        self.settings.nodes[index].1[3]=(target.point.z-c.profile_point(node[0]*length,x,self.settings.height).z).clamp(-50.0,50.0);
        let step=(0.5/length).min(0.1);let t=if node[0]==0.0{step}else{1.0-step};
        let near=if let Some(i)=self.settings.nodes.iter().position(|(lane,n)|*lane==pi&&(n[0]-t).abs()<1e-9){i}else{
            if self.settings.nodes.len()>=128{return false;}
            self.settings.nodes.push((pi,[t,0.0,0.0,0.0]));self.settings.nodes.len()-1
        };
        let neighbor=target.point+target.direction*(length*step)*if start{1.0}else{-1.0};
        self.move_node(near,neighbor);
        self.settings.nodes[near].1[3]=(neighbor.z-c.profile_point(t*length,x,self.settings.height).z).clamp(-50.0,50.0);
        self.settings.connections.retain(|(lane,out,_)|*lane!=pi||*out==start);
        self.settings.connections.push((pi,!start,[target.key.tile.0 as i64,target.key.tile.1 as i64,target.key.id,target.key.path as i64,i64::from(target.reversed)]));
        true
    }
    pub fn move_node(&mut self,index:usize,target:DVec3) {
        if let Some(start)=self.node_is_start(index) {let pi=self.settings.nodes[index].0;self.settings.connections.retain(|(lane,out,_)|*lane!=pi||*out==start);}
        let Some((tile,piece))=self.preview.pieces.first() else{return;};
        let Some((pi,node))=self.settings.nodes.get(index).copied() else{return;};
        let Some((x,_,_))=lanes(&self.settings,false).get(pi).copied() else{return;};
        let def=omsi_scenery::Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,&self.preview.definitions[0].1));
        let c=SplineCurve::from_map(piece,DVec2::new(tile.0 as f64,tile.1 as f64)*omsi_map::tile_size()).with_sli(&def);
        let station=node[0]*piece.length;let base=c.profile_point(station,x,self.settings.height);
        let forward=SplineCurve::dir(c.heading_at(station)).extend(0.0);let right=DVec3::new(forward.y,-forward.x,0.0);
        let delta=target-base;let n=&mut self.settings.nodes[index].1;
        n[1]=delta.dot(right).clamp(-50.0,50.0);n[2]=delta.dot(forward).clamp(-50.0,50.0);
    }
    pub fn hit(&self,p:(f32,f32))->Option<Command>{self.rects.iter().rev().find(|(r,_)|p.0>=r[0]&&p.0<=r[2]&&p.1>=r[1]&&p.1<=r[3]).map(|(_,c)|*c)}
    pub fn refresh(&mut self,world:&World){
        let _diagnostic = crate::editor_diagnostics::Span::new("ai_preview", format!("tile={:?} spline={} nodes={} targets={} connected={} replace={}", self.start.0,self.start.1,self.settings.nodes.len(),self.targets.len(),self.settings.connected,self.replace));
        let previous_error=self.error.clone();let previous_message=self.preview.message.clone();
        self.preview=Plan::default();
        self.error=(||{
            if self.settings.connected&&!self.settings.nodes.is_empty(){return Err("Freiformpunkte sind lokal: Bereich auf ausgewählten Spline stellen".into());}
            let roads=crate::roadside_objects::traffic_route(world,self.start,self.settings.connected)?;
            let keys:Vec<_>=roads.iter().map(|r|r.0).collect();
            let mut occupied=crate::roadside_objects::generated_traffic_sources(world,&keys);
            for (key,s,_) in &roads {
                let ty=world.spline_type(&s.file).ok_or("Splineprofil fehlt")?;
                if ty.def.paths.iter().any(|p|p.kind==0) {occupied.insert(*key);}
            }
            let skipped=occupied.len();
            if self.replace {
                let existing=crate::roadside_objects::generated_traffic(world,&keys);
                for (key,source,backwards) in &roads {
                    let Some(items)=existing.get(key) else{continue;};
                    if items.len()!=1 {return Err(format!("Spline {} hat mehrere erzeugte KI-Splines. Zuordnung zuerst prüfen.",key.1));}
                    if world.spline_type(&source.file).is_some_and(|ty|ty.def.paths.iter().any(|p|p.kind==0)) {
                        return Err("Referenz besitzt eigene Fahrzeugpfade; Ersetzen hier gesperrt".into());
                    }
                    let (old_key,old)=&items[0];
                    if old_key.0!=key.0{return Err("KI-Pfad liegt auf einer anderen Kachel als seine Referenz; Zuordnung prüfen".into());}
                    let mut local=self.settings.clone();
                    if self.settings.connected {
                        if let Some(ty)=world.spline_type(&old.file) {if !ty.def.editor_path_nodes.is_empty(){return Err("Kette enthält Freiformpfade. Diese bitte einzeln bearbeiten.".into());}}
                    }
                    if !self.settings.connected {local.nodes=self.settings.nodes.clone();}
                    let mut part=build(&[(*key,source.clone(),*backwards)],&local)?;
                    let after=&mut part.pieces[0].1;
                    after.id=old.id;after.prev_id=old.prev_id;after.next_id=old.next_id;
                    // Preserve unrelated per-path rules, e.g. speed limits; removed lanes lose their rules.
                    after.rules.extend(old.rules.iter().filter(|r|r.path_index>=0&&(r.path_index as usize)<if self.settings.two_way{2}else{1}
                        && !["bus","trucks","no_cars","editor_bus_only","editor_surface_path"].contains(&r.kind.as_str())).cloned());
                    self.preview.replacements.push((*old_key,old.clone(),after.clone()));
                    self.preview.pieces.append(&mut part.pieces);self.preview.definitions.append(&mut part.definitions);
                    self.preview.handles.append(&mut part.handles);self.preview.markers.append(&mut part.markers);self.preview.endpoints.append(&mut part.endpoints);
                }
                if self.preview.pieces.is_empty(){return Err("Keine mit diesem Werkzeug erzeugten KI-Pfade im gewählten Bereich. Zum Anlegen auf Ergänzen umschalten.".into());}
                if self.preview.markers.len()>100000{return Err("Vorschau zu groß: kürzere Kette wählen".into());}
            } else {self.preview=if skipped==roads.len() {Plan::default()} else {build_missing(&roads,&self.settings,&occupied)?};}
            // Conservative endpoint diagnostic. Interior junctions need explicit connectors.
            let lanes=world.lanes.lock();let mut open=0;
            for (i,(p,d,start)) in self.preview.endpoints.iter().enumerate() {
                let internal=self.preview.endpoints.iter().enumerate().any(|(j,(q,e,other))|i!=j&&start!=other&&p.distance(*q)<0.5&&d.dot(*e)>0.8);
                let external=self.targets.iter().any(|t|*start!=t.start&&p.distance(t.point)<0.5&&d.dot(t.direction)>0.8)
                    ||lanes.iter().filter(|l|l.kind==omsi_sim::traffic::LaneKind::Street).any(|l|{
                    let q=if *start {l.end()}else{l.start()};let h=if *start {l.end_heading()}else{l.start_heading()};
                    p.distance(q)<0.5&&d.dot(SplineCurve::dir(h as f64).extend(0.0))>0.8
                });
                if !internal&&!external{open+=1;}
            }
            self.preview.message=format!("{} neue Abschnitte · {} bereits mit Pfaden (unverändert) · {} offene Pfadenden. Anschlüsse prüfen; nach Speichern Karte neu laden.",self.preview.pieces.len(),skipped,open);
            if self.replace {self.preview.message=format!("{} KI-Abschnitte ersetzen · {} feste Anschlüsse · {} offene Pfadenden. Enter übernimmt; danach Strg+Z rückgängig.",self.preview.replacements.len(),self.settings.connections.len(),open);}
            Ok::<(),String>(())
        })().err();
        if let Some(error)=&self.error {if previous_error.as_ref()!=Some(error) {
            log::warn!("AI path preview for spline {} (connected={}): {}",self.start.1,self.settings.connected,error);
        }} else if previous_error.is_some()||previous_message!=self.preview.message {log::info!("AI path preview for spline {}: {}",self.start.1,self.preview.message);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlay_discards_results_from_before_invalidation() {
        let mut overlay=Overlay::default();
        let old=overlay.generation;
        overlay.invalidate();
        overlay.accept((old,DVec3::ZERO,Vec::new()));
        assert!(overlay.center.is_none());
        overlay.accept((overlay.generation,DVec3::X,Vec::new()));
        assert_eq!(overlay.center,Some(DVec3::X));
        overlay.invalidate();
        assert!(overlay.center.is_none());
    }

    #[test]
    fn invisible_paths_preserve_reference_geometry_and_reversed_chain() {
        let original=MapSpline {id:42,file:"Splines/terrain.sli".into(),length:40.0,radius:90.0,
            heading:27.0,grad_start:1.0,grad_end:3.0,is_h:true,delta_h:Some(0.8),
            terrain_align:Some(8.0),terrain_align_flag:true,mirror:true,..Default::default()};
        let snapshot=original.clone();
        for backwards in [false,true] {for reverse in [false,true] {for two_way in [false,true] {
            let s=Settings {reverse,two_way,forward_bus:true,..Default::default()};
            let p=build(&[(((0,0),42),original.clone(),backwards)],&s).unwrap();
            let piece=&p.pieces[0].1;
            let def=omsi_scenery::Spline::parse(&omsi_cfg::CfgFile::from_str(&piece.file,&p.definitions[0].1));
            assert!(def.profiles.is_empty());assert_eq!(def.paths.len(),if two_way{2}else{1});
            for (path,(x,d,_)) in def.paths.iter().zip(lanes(&s,backwards)) {
                assert!((path.start[0] as f64-x).abs()<1e-5);assert_eq!(path.direction,d);
            }
            assert_eq!(piece.length,original.length);assert_eq!(piece.radius,original.radius);
            assert_eq!(piece.grad_start,original.grad_start);assert_eq!(piece.delta_h,original.delta_h);
            assert!(!piece.terrain_align_flag && piece.terrain_align.is_none());
            let a=SplineCurve::from_map(&original,DVec2::ZERO);let b=SplineCurve::from_map(piece,DVec2::ZERO);
            for t in [0.0,0.25,0.5,1.0] {assert!(a.point_at(t*40.0).distance(b.point_at(t*40.0))<1e-9);}
        }}}
        assert_eq!(original,snapshot);
    }
    #[test]
    fn reducing_to_one_lane_keeps_first_lane_position_and_direction() {
        let mut s=Settings {offset:2.0,reverse:true,..Default::default()};
        let original=lanes(&s,false)[0];s.toggle_two_way();
        assert_eq!(lanes(&s,false),vec![original]);
        s.toggle_two_way();assert_eq!(lanes(&s,false)[0],original);
    }
    #[test]
    fn connected_chain_fills_only_missing_sections() {
        let roads:Vec<_>=(1..=5).map(|id|(((0,0),id),MapSpline {id,length:20.0,pos:[0.0,(id-1) as f64*20.0,0.0],..Default::default()},false)).collect();
        let occupied=hashbrown::HashSet::from([((0,0),2),((0,0),4)]);
        let p=build_missing(&roads,&Settings::default(),&occupied).unwrap();
        assert_eq!(p.pieces.len(),3);assert_eq!(p.definitions.len(),3);
        assert_eq!(p.pieces.iter().map(|(_,s)|s.pos[1]).collect::<Vec<_>>(),vec![0.0,40.0,80.0]);
        let all=roads.iter().map(|r|r.0).collect();
        assert!(build_missing(&roads,&Settings::default(),&all).is_err());
    }
    #[test]
    fn rejects_gaps_and_invalid_geometry() {
        let s=Settings::default();let road=MapSpline {length:20.0,..Default::default()};
        let mut next=road.clone();next.pos[1]=21.0;
        assert!(build(&[(((0,0),1),road.clone(),false),(((0,0),2),next,false)],&s).is_err());
        let mut canted=road.clone();canted.cant_end=2.0;
        assert!(build(&[(((0,0),1),canted,false)],&s).is_ok());
        let mut broken=road;broken.heading=f64::NAN;
        assert!(build(&[(((0,0),1),broken,false)],&s).is_err());
    }
}
