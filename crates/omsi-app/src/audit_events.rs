//! Async, read-only whole-map audit and navigation to its results.
use crate::spline_audit::{self,Report,Object};
use glam::DVec3;
use std::{sync::{Arc,mpsc},collections::{HashSet,BTreeMap},path::{Path,PathBuf}};

#[derive(Clone,Copy)]
pub enum Command {Scan,ToggleMarkers,ToggleGood,Previous,Next,Jump(usize),Close}
pub struct Audit {
    pub visible:bool,pub markers:bool,pub show_good:bool,pub page:usize,
    pub report:Report,pub message:String,pub rect:Option<[f32;4]>,pub buttons:Vec<([f32;4],Command)>,
    receiver:Option<mpsc::Receiver<Report>>,
}
impl Default for Audit {fn default()->Self{Self{visible:true,markers:true,show_good:false,page:0,report:Report::default(),message:String::new(),rect:None,buttons:Vec::new(),receiver:None}}}
impl Audit {
    pub fn busy(&self)->bool{self.receiver.is_some()}
    pub fn poll(&mut self){if let Some(rx)=&self.receiver{match rx.try_recv(){
        Ok(report)=>{self.report=report;self.receiver=None;self.page=0;self.message="Results of the last check – check again after changes".into();},
        Err(mpsc::TryRecvError::Disconnected)=>{self.receiver=None;self.message="Check cancelled – check again".into();},
        Err(mpsc::TryRecvError::Empty)=>{}
    }}}
    pub fn indices(&self)->std::ops::Range<usize>{0..self.report.visible_len(self.show_good)}
    pub fn hit(&self,p:(f32,f32))->Option<Command>{self.buttons.iter().rev().find(|(r,_)|contains(*r,p)).map(|(_,c)|*c)}
}
pub fn contains(r:[f32;4],p:(f32,f32))->bool{p.0>=r[0]&&p.0<=r[2]&&p.1>=r[1]&&p.1<=r[3]}
pub fn panel_active(ed:&crate::editor::Editor)->bool{
    ed.catalog.is_none()&&ed.junction_window.is_none()&&ed.sidewalk_window.is_none() && ed.traffic_window.is_none()&&ed.roadside_window.is_none()&&ed.tile_window.is_none()&&ed.text_window.is_none()
}
#[derive(Clone)]
struct AddedObject {id:i64,path:PathBuf,at:DVec3,heading:f64,tilt:[f64;2],deleted:bool}

/// Ports are provable only for upright, absolute-height exported builder objects.
fn ports(path:&Path,at:DVec3,heading:f64,tilt:[f64;2])->Option<Vec<DVec3>>{
    if !at.is_finite()||!heading.is_finite()||tilt.iter().any(|v|!v.is_finite()||v.abs()>1e-6){return None;}
    let bytes=std::fs::read(path.parent()?.join("junction.junction.json")).ok()?;
    if bytes.len()>65536{return None;}
    let p:crate::junction_builder::Project=serde_json::from_slice(&bytes).ok()?;
    let cfg=omsi_cfg::CfgFile::read(path).ok()?;
    if !omsi_scenery::SceneryObject::parse(&cfg).absolute_height(){return None;}
    let(s,c)=heading.to_radians().sin_cos();
    Some((0..4).filter_map(|i|crate::junction_builder::port(&p,i).ok()).map(|(p,_,_)|at+DVec3::new(p.x*c+p.y*s,p.y*c-p.x*s,p.z)).collect())
}
fn collect(world:Arc<crate::scene::World>,edits:crate::spline_editor::Edits,object_edits:hashbrown::HashMap<i64,crate::scene::ObjectEdit>,added:Vec<AddedObject>,saved:hashbrown::HashMap<(i32,i32),Vec<u8>>)->Report{
    let mut tiles:BTreeMap<_,_>=world.global.tiles.iter().map(|t|((t.x,t.y),omsi_cfg::resolve_path(&world.map_dir,&t.file))).collect();
    for(_,x,y,path)in world.map_tiles(){tiles.insert((x,y),path);}
    let chrono=world.chrono_dirs.read().clone();let size=omsi_map::tile_size();
    let mut roads=Vec::new();let mut records=Vec::new();let mut unread=Vec::new();let mut seen=HashSet::new();
    for(key,path)in tiles {
        let Some(mut tile)=crate::tiles::read_tile(&path,&chrono)else{unread.push(key);continue;};
        edits.overlay(key,&mut tile);
        for s in tile.splines.drain(..){seen.insert((key,s.id));roads.push(((key,s.id),s));}
        if let Some(bytes)=saved.get(&key){
            let original=omsi_map::Tile::parse(&omsi_cfg::CfgFile::from_bytes(&path,bytes));
            spline_audit::restore_object_baseline(&mut tile,&original,|id|object_edits.contains_key(&id));
        }
        records.extend(tile.objects.into_iter().map(|o|(key,o)));
    }
    // Newly created splines in a tile whose backing file could not be read still have
    // known endpoints; do not pretend the rest of that tile has been checked.
    for(&key,s)in &edits.added{if seen.insert(key){roads.push((key,s.clone()));}}
    let referenced:HashSet<_>=roads.iter().flat_map(|(_,s)|[s.prev_id,s.next_id]).filter(|id|*id!=0).collect();
    let replacements:HashSet<_>=added.iter().map(|a|a.id).collect();
    let mut objects=Vec::new();
    for(key,o)in records {
        if replacements.contains(&o.id){continue;}
        let e=object_edits.get(&o.id).copied().unwrap_or_default();
        let at=DVec3::from_array(o.pos)+DVec3::new(key.0 as f64*size,key.1 as f64*size,0.0)+e.moved;
        let p=if !e.deleted&&o.parent_id.is_none()&&referenced.contains(&o.id){ports(&omsi_cfg::resolve_path(&world.root,&o.file),at,o.rot[0]+e.turned,[o.rot[1],o.rot[2]])}else{None};
        objects.push(Object{id:o.id,deleted:e.deleted,ports:p});
    }
    for a in added{let p=if !a.deleted&&referenced.contains(&a.id){ports(&a.path,a.at,a.heading,a.tilt)}else{None};objects.push(Object{id:a.id,deleted:a.deleted,ports:p});}
    spline_audit::inspect(&roads,&objects,size,unread)
}
impl crate::app::App {
    pub(crate) fn editor_open_audit(&mut self){
        if self.world.as_ref().is_none_or(|w|w.global.world_coordinates){self.service_msg=Some(("Spline check requires a standard OMSI map".into(),5.0));return;}
        self.editor_terrain_finish();
        let Some(ed)=self.menus.editor.as_mut()else{return;};
        ed.splines.finish_drag();ed.end_object_drag();
        let first=ed.audit.is_none();let a=ed.audit.get_or_insert_with(Audit::default);a.visible=true;
        self.menus.editor_drag=false;self.input.buttons_held=(false,false);self.input.keys.clear();
        if first{self.editor_audit_command(Command::Scan);}
    }
    pub(crate) fn editor_audit_command(&mut self,command:Command){
        let(Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref())else{return;};
        let saved=if matches!(command,Command::Scan){ed.audit_saved_sources()}else{Default::default()};
        let Some(a)=ed.audit.as_mut()else{return;};
        match command {
            Command::Scan=>{
                if a.busy(){return;}
                let edits=world.spline_edits.lock().clone();let object_edits=world.object_edits.lock().clone();
                let added=ed.added.iter().map(|o|AddedObject{id:o.id,path:o.sco.clone(),at:o.base+o.moved,heading:o.base_heading+o.turned,tilt:o.tilt,deleted:o.deleted}).collect();
                let world=world.clone();let(tx,rx)=mpsc::channel();a.receiver=Some(rx);a.report=Report::default();a.page=0;a.message="Checking all tiles in the background …".into();
                if let Err(e)=std::thread::Builder::new().name("spline-audit".into()).spawn(move||{let report=collect(world,edits,object_edits,added,saved);let _=tx.send(report);}){
                    a.receiver=None;a.message=format!("Could not start check: {e}");
                }
            }
            Command::ToggleMarkers=>a.markers=!a.markers,
            Command::ToggleGood=>{a.show_good=!a.show_good;a.page=0;},
            Command::Previous=>a.page=a.page.saturating_sub(1),
            Command::Next=>a.page=(a.page+1).min(a.indices().len().saturating_sub(1)/8),
            Command::Close=>a.visible=false,
            Command::Jump(i)=>{
                let Some(e)=a.report.entries.get(i).filter(|e|e.at.is_finite())else{return;};
                if let Some(cam)=self.camera.as_mut(){cam.position=e.at+DVec3::new(0.0,-18.0,20.0);cam.yaw=0.0;cam.pitch=-48.0;cam.roll=0.0;}
                self.view="free".into();self.cam.ego=false;self.session.on_foot=None;self.input.keys.clear();self.menus.editor_drag=false;self.input.buttons_held=(false,false);
                ed.spline_mode=true;ed.terrain.active=false;ed.splines.selected=Some(e.key);
                self.service_msg=Some((format!("Spline {} · {} · {} · {}",e.key.1,if e.end==0{"Start"}else{"End"},e.reason,e.file),15.0));
            }
        }
    }
}
