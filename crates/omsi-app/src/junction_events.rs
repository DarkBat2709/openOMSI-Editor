//! Modal junction builder and shared texture-catalogue routing.
use crate::app::App;
use crate::editor::{ObjectStamp,TextureTarget};
use crate::junction_builder::{Command,Field};
use winit::keyboard::KeyCode;
use std::path::Path;

impl App {
    pub(crate) fn editor_open_textures(&mut self,target:TextureTarget) {
        if self.net.lan.is_some() {self.service_msg=Some(("Texture editing is available in single-player".into(),5.0));return;}
        self.editor_terrain_finish();
        let (Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref()) else {return;};
        ed.texture_target=Some(target);ed.tile_window=None;ed.text_window=None;ed.roadside_window=None;ed.sidewalk_window=None;ed.traffic_window=None;ed.placing_asset=None;ed.object_stamp=None;
        ed.splines.cancel_connection();ed.splines.cancel_generation();ed.splines.finish_drag();ed.end_object_drag();
        ed.catalog=Some(crate::asset_catalog::Catalog::with_map(world.root.clone(),crate::asset_catalog::Kind::Texture,Some(world.map_dir.clone())));
        self.menus.editor_drag=false;self.input.keys.clear();self.input.mouse_look=false;self.input.mmb_held=false;self.input.buttons_held=(false,false);
        if let Some(window)=self.window.as_ref() {window.set_cursor_visible(true);}
    }

    pub(crate) fn editor_open_junction(&mut self) {self.editor_open_builder(false);}
    pub(crate) fn editor_open_roundabout(&mut self) {self.editor_open_builder(true);}
    fn editor_open_builder(&mut self,roundabout:bool) {
        if self.net.lan.is_some() {self.service_msg=Some(("Junction builder is available in single-player".into(),5.0));return;}
        self.editor_terrain_finish();
        let (Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref()) else {return;};
        ed.splines.finish_drag();ed.end_object_drag();ed.splines.cancel_connection();ed.splines.cancel_generation();
        ed.catalog=None;ed.texture_target=None;ed.tile_window=None;ed.text_window=None;ed.roadside_window=None;ed.sidewalk_window=None;ed.traffic_window=None;ed.terrain.active=false;
        ed.placing_asset=None;ed.object_stamp=None;
        ed.junction_window=Some(if roundabout {crate::junction_builder::Window::new_roundabout(world.global.left_hand_traffic)} else {crate::junction_builder::Window::new(world.global.left_hand_traffic)});
        let target=ed.editing_added.and_then(|i|ed.added.get(i)).map(|o|o.id).or(ed.selected);
        let selected_sco=ed.editing_added.and_then(|i|ed.added.get(i)).map(|o|o.sco.clone())
            .or_else(||ed.selected.and_then(|id|world.edit_objects.lock().get(&id).map(|o|o.sco.clone())));
        if let Some(path)=selected_sco.and_then(|p|p.parent().map(|dir|dir.join("junction.junction.json"))).filter(|p|p.is_file()) {
            let same_kind=std::fs::read(&path).ok().and_then(|b|serde_json::from_slice::<crate::junction_builder::Project>(&b).ok()).is_some_and(|p|p.roundabout.is_some()==roundabout);
            let w=ed.junction_window.as_mut().unwrap();
            if same_kind {match w.load(&path){Ok(())=>{w.target=target;w.placed_project=Some(w.project.clone());w.message="Editing placed junction · Update keeps position and ID. Road connections are checked too.".into();},Err(e)=>w.message=e}}
        }
        let source_label=ed.splines.selected.and_then(|key|world.spline_edits.lock().current(key).map(|s|format!("Selected road: {} · {}",key.1,s.file))).unwrap_or_else(||"No road selected – select one in spline mode before opening the builder".into());
        ed.junction_window.as_mut().unwrap().source_label=source_label;
        if let Some(id)=target.filter(|_|ed.junction_window.as_ref().unwrap().target.is_some()){
            let project=ed.junction_window.as_ref().unwrap().project.clone();
            match junction_roads(ed,world,id,&project,&project,None){Ok(parts)=>{let _=show_junction_connections(ed,world,id,&project,&parts,None);},Err(e)=>{ed.junction_window.as_mut().unwrap().message=format!("Check existing connections: {e}");}}
        }
        self.menus.editor_drag=false;self.input.dragging=false;self.input.keys.clear();self.input.mouse_look=false;self.input.mmb_held=false;self.input.buttons_held=(false,false);
        if let Some(window)=self.window.as_ref() {window.set_cursor_visible(true);}
        if let Some(ui)=self.ui.as_mut() {ui.end_editor_dock_drag();}
    }

    pub(crate) fn editor_junction_command(&mut self,command:Command) {
        if !self.menus.editor.as_ref().is_some_and(|ed|ed.junction_window.is_some()) {return;}
        match command {
            Command::Close=>{let ed=self.menus.editor.as_mut().unwrap();ed.junction_window=None;ed.texture_target=None;self.input.keys.clear();},
            Command::RoadTexture|Command::WalkTexture=>{
                if !self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap().commit() {return;}
                self.editor_open_textures(if matches!(command,Command::WalkTexture) {TextureTarget::JunctionWalk} else {TextureTarget::JunctionRoad});
            }
            Command::Load=>{
                let mut dialog=rfd::FileDialog::new().set_title(omsi_ui::tr("Load junction project").as_ref()).add_filter(omsi_ui::tr("Junction project").as_ref(),&["json"]);
                if let Some(content)=crate::startup::content_dir() {dialog=dialog.set_directory(content.join("Sceneryobjects/openOMSI_Editor/Junctions"));}
                if let Some(path)=dialog.pick_file() {
                    let window=self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap();
                    if let Err(e)=window.load(&path) {window.message=format!("Not loaded: {e}");}
                }
                self.input.keys.clear();self.input.buttons_held=(false,false);
            }
            Command::UseSpline=>{
                let Some(world)=self.world.as_ref() else {return;};
                let selected=self.menus.editor.as_ref().and_then(|ed|ed.splines.selected);
                let spline=selected.and_then(|key|world.spline_edits.lock().current(key));
                let outcome=(||->Result<(f64,String,crate::junction_builder::RoadSurface,f64),String> {
                    let spline=spline.ok_or("Select a road in spline mode first, then open the builder")?;
                    let ty=world.spline_type(&spline.file).ok_or("Road profile missing")?;
                    let paths:Vec<_>=ty.def.paths.iter().filter(|p|p.kind==0).collect();
                    if paths.is_empty() {return Err("This spline has no road paths".into());}
                    let lo=paths.iter().map(|p|p.start[0] as f64-p.width as f64/2.0).fold(f64::INFINITY,f64::min);
                    let hi=paths.iter().map(|p|p.start[0] as f64+p.width as f64/2.0).fold(f64::NEG_INFINITY,f64::max);
                    let dirs=crate::scene::texture_dirs(&world.root,&ty.dir);let refs:Vec<&Path>=dirs.iter().map(|p|p.as_path()).collect();
                    let (slot,surface,metres)=crate::junction_builder::spline_surface(&ty.def,spline.mirror,spline.length)?;
                    let texture=ty.def.textures.get(slot).and_then(|t|omsi_texture::find_texture(&t.file,&refs)).and_then(|p|{
                        let mut roots=omsi_cfg::content_roots();roots.push(world.root.clone());
                        roots.iter().filter_map(|root|p.strip_prefix(root).ok()).max_by_key(|p|p.components().count())
                            .map(|p|p.to_string_lossy().replace('\\',"/"))
                    });
                    if !(4.0..=30.0).contains(&(hi-lo)) {return Err("Road width is outside 4–30 m".into());}
                    Ok((hi-lo,texture.ok_or("Road texture missing or outside the content folders")?,surface,metres))
                })();
                let window=self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap();
                if !window.commit() {return;}match outcome {Ok((width,texture,surface,metres))=>window.use_spline(width,texture,surface,metres),Err(e)=>window.message=e}
            }
            Command::ConnectRoad=>{
                let(Some(world),Some(ed))=(self.world.as_ref(),self.menus.editor.as_mut())else{return;};
                let w=ed.junction_window.as_mut().unwrap();if !w.commit(){return;}
                let pending=ed.splines.selected.map(|key|(key,w.arm));
                let target=w.target;let project=w.project.clone();let old=w.placed_project.clone();
                let result=(||{let id=target.ok_or("Select a placed builder junction in object mode first")?;
                    let pending=pending.ok_or("Before switching to object mode, select the road to connect in spline mode")?;
                    junction_roads(ed,world,id,&project,old.as_ref().unwrap_or(&project),Some(pending))})();
                match result{Ok(roads)=>{
                    let id=target.unwrap();let shown=show_junction_connections(ed,world,id,&project,&roads,pending);
                    let w=ed.junction_window.as_mut().unwrap();match shown{Ok(())=>{w.pending=pending;w.message=format!("Green: existing connections · Blue: prepared for arm {}. Update applies both; Close discards preparation.",['A','B','C','D'][w.arm]);},Err(e)=>w.message=e}
                },Err(e)=>{let w=ed.junction_window.as_mut().unwrap();w.clear_connection_preview();w.message=e;}}

            }
            Command::Export=>{
                let window=self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap();if !window.commit() {return;}
                let project=window.project.clone();let Some(world)=self.world.as_ref() else {return;};
                let Some(content)=crate::startup::content_dir() else {window.message="No writable content folder available".into();return;};
                let target=window.target;let old=window.placed_project.clone();let pending=window.pending;
                let roads=if let Some(id)=target{match junction_roads(self.menus.editor.as_ref().unwrap(),world,id,&project,old.as_ref().unwrap_or(&project),pending){Ok(v)=>v,Err(e)=>{self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap().message=format!("Not updated: {e}");return;}}}else{Vec::new()};
                match crate::junction_builder::export(&project,&world.root,&content,&self.args.root) {
                    Ok(path)=>{
                        omsi_cfg::content_changed();
                        if let Some(id)=target{
                            let(Some(r),Some(scene))=(self.renderer.as_ref(),self.scene.as_mut())else{return;};
                            let ed=self.menus.editor.as_mut().unwrap();
                            match ed.replace_junction(world,r,scene,id,path,roads,true){Ok(())=>{ed.junction_window=None;ed.spline_mode=false;self.editor_reload_splines();self.service_msg=Some(("Junction and connections updated · Position/ID kept · Undo in object mode · Ctrl+S to save".into(),12.0));},Err(e)=>ed.junction_window.as_mut().unwrap().message=e}
                            return;
                        }
                        let file=path.strip_prefix(&content).unwrap_or(&path).to_string_lossy().replace('\\',"/");
                        let ed=self.menus.editor.as_mut().unwrap();ed.junction_window=None;ed.spline_mode=false;ed.align_object=false;
                        ed.start_object(ObjectStamp::new(path.clone()));ed.align_object=false;
                        ed.placing_asset=Some(crate::asset_catalog::Asset {kind:crate::asset_catalog::Kind::Object,file,
                            name:project.name,path,category:crate::asset_catalog::Category::Junctions,groups:"Custom junctions".into()});
                        self.service_msg=Some(("Junction saved · Click target · N/M to rotate · U/O for height · Ctrl+S saves placement".into(),12.0));
                        self.input.keys.clear();self.input.buttons_held=(false,false);
                    }
                    Err(e)=>self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap().message=format!("Not saved: {e}"),
                }
            }
            command=>self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap().command(command),
        }
    }

    /// A modal builder owns text and shortcuts; held camera keys are cleared on entry.
    pub(crate) fn editor_junction_input(&mut self,code:Option<KeyCode>,text:Option<&str>,pressed:bool)->bool {
        if !self.menus.editor.as_ref().is_some_and(|ed|ed.junction_window.is_some() && ed.catalog.is_none()) {return false;}
        if let Some(key)=code {
            if matches!(key,KeyCode::ControlLeft|KeyCode::ControlRight|KeyCode::ShiftLeft|KeyCode::ShiftRight) {
                if pressed {self.input.keys.insert(key);} else {self.input.keys.remove(&key);}return true;
            }
            self.input.keys.remove(&key);
        }
        if !pressed {return true;}
        let ctrl=self.input.keys.contains(&KeyCode::ControlLeft)||self.input.keys.contains(&KeyCode::ControlRight);
        let shift=self.input.keys.contains(&KeyCode::ShiftLeft)||self.input.keys.contains(&KeyCode::ShiftRight);
        let editing=self.menus.editor.as_ref().unwrap().junction_window.as_ref().unwrap().input.is_some();
        let command=match code {
            Some(KeyCode::Escape) if !editing=>Some(Command::Close),
            Some(KeyCode::Enter|KeyCode::NumpadEnter) if !editing=>Some(Command::Export),
            Some(KeyCode::KeyZ) if ctrl=>Some(if shift {Command::Redo} else {Command::Undo}),
            Some(KeyCode::KeyY) if ctrl=>Some(Command::Redo),
            _=>None,
        };
        if let Some(command)=command {self.editor_junction_command(command);return true;}
        let window=self.menus.editor.as_mut().unwrap().junction_window.as_mut().unwrap();
        match code {
            Some(KeyCode::Escape)=>window.input=None,
            Some(KeyCode::Enter|KeyCode::NumpadEnter)=>{window.commit();},
            Some(KeyCode::KeyA) if ctrl=>{if let Some(input)=window.input.as_mut() {input.replace=true;}},
            Some(KeyCode::Backspace)=>{if let Some(input)=window.input.as_mut() {if input.replace {input.text.clear();} else {input.text.pop();}input.replace=false;}},
            Some(KeyCode::Delete)=>{if let Some(input)=window.input.as_mut() {input.text.clear();input.replace=false;}},
            _=>{if !ctrl {if let (Some(input),Some(text))=(window.input.as_mut(),text) {
                let typed:String=text.chars().filter(|c|!c.is_control() && (input.field==Field::Name || c.is_ascii_digit()||matches!(*c,'-'|'+'|','|'.'))).collect();
                if !typed.is_empty() {if input.replace {input.text.clear();input.replace=false;}
                    if input.text.chars().count()+typed.chars().count()<=80 {input.text.push_str(&typed);}}
            }}},
        }true
    }
}

/// Resolve explicit links; cached originals avoid reading every map tile on each drag frame.
fn linked_roads(world:&crate::scene::World,id:i64)->Result<Vec<(crate::spline_editor::Key,omsi_map::MapSpline)>,String>{
    let index=world.index();
    let missing={let edits=world.spline_edits.lock();index.splines.iter().filter(|(_,s)|s.prev==id||s.next==id)
        .filter(|(sid,_)|!edits.originals.keys().chain(edits.changed.keys()).chain(edits.added.keys()).any(|k|k.1==**sid)).count()};
    if missing>0 {for(_,tx,ty,_)in world.map_tiles(){let(tile,editable)=world.editor_row_source((tx,ty))?;
        if tile.version!=0&&tile.version<14{continue;}
        let mut edits=world.spline_edits.lock();for s in tile.splines.into_iter().filter(|s|editable.contains(&s.id)){edits.originals.entry(((tx,ty),s.id)).or_insert(s);}
    }}
    let edits=world.spline_edits.lock();
    if index.splines.iter().filter(|(_,s)|s.prev==id||s.next==id).any(|(sid,_)|!edits.originals.keys().chain(edits.changed.keys()).chain(edits.added.keys()).any(|k|k.1==*sid)){
        return Err("A connected road cannot be edited (missing, Chrono scenario or old tile format)".into());
    }
    let keys:std::collections::BTreeSet<_>=edits.originals.iter().chain(edits.changed.iter()).chain(edits.added.iter()).filter(|(_,s)|s.prev_id==id||s.next_id==id).map(|(k,_)|*k).collect();
    Ok(keys.into_iter().filter_map(|k|edits.current(k).filter(|s|!s.deleted&&(s.prev_id==id||s.next_id==id)).map(|s|(k,s))).collect())
}

pub(crate) fn follow_junction_pose(ed:&crate::editor::Editor,world:&crate::scene::World,id:i64,new:crate::junction_connections::Pose)->Result<Vec<(crate::spline_editor::Key,omsi_map::MapSpline)>,String>{
    let (path,at,heading,tilt,_,_)=ed.junction_placement(world,id)?;
    if !crate::editor::builder_asset(&path){return Ok(Vec::new());}
    let path=path.parent().unwrap().join("junction.junction.json");
    let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
    if bytes.len()>65536{return Err("Junction project is too large".into());}
    let project:crate::junction_builder::Project=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    junction_roads_at(ed,world,id,&project,&project,None,crate::junction_connections::Pose{at,heading,tilt},new)
}
fn junction_roads(ed:&crate::editor::Editor,world:&crate::scene::World,id:i64,project:&crate::junction_builder::Project,old:&crate::junction_builder::Project,pending:Option<(crate::spline_editor::Key,usize)>)->Result<Vec<(crate::spline_editor::Key,omsi_map::MapSpline)>,String>{
    let(_,at,heading,tilt,_,_)=ed.junction_placement(world,id)?;
    let pose=crate::junction_connections::Pose{at,heading,tilt};
    junction_roads_at(ed,world,id,project,old,pending,pose,pose)
}
fn junction_roads_at(ed:&crate::editor::Editor,world:&crate::scene::World,id:i64,project:&crate::junction_builder::Project,old:&crate::junction_builder::Project,pending:Option<(crate::spline_editor::Key,usize)>,before:crate::junction_connections::Pose,after:crate::junction_connections::Pose)->Result<Vec<(crate::spline_editor::Key,omsi_map::MapSpline)>,String>{
    if world.global.world_coordinates{return Err("Junction connections require a standard OMSI map".into());}
    let linked=linked_roads(world,id)?;
    if linked.is_empty()&&pending.is_none(){return Ok(Vec::new());}
    let old_ports:Vec<_>=(0..4).filter_map(|i|crate::junction_builder::port(old,i).ok().map(|(q,_,_)|(i,before.point(q)))).collect();
    let mut assigned=std::collections::BTreeMap::new();
    for(key,s)in linked {
        if pending.is_some_and(|(k,_)|k==key){continue;}
        if s.prev_id==id&&s.next_id==id{return Err("Road is connected to this junction at both ends; edit separately".into());}
        let curve=omsi_geometry::SplineCurve::from_map(&s,glam::DVec2::new(key.0.0 as f64,key.0.1 as f64)*omsi_map::tile_size());
        let p=curve.point_at(if s.prev_id==id{0.0}else{s.length});
        let arm=old_ports.iter().map(|(i,q)|(*i,q.distance(p))).min_by(|a,b|a.1.total_cmp(&b.1)).filter(|(_,d)|*d<0.1).ok_or("Road no longer meets its arm: select that road and reconnect it in the builder")?.0;
        assigned.insert(key,arm);
    }
    if let Some((key,arm))=pending{assigned.insert(key,arm);}
    let mut occupied=std::collections::HashSet::new();let mut result=Vec::new();
    for(key,arm)in assigned{if !occupied.insert(arm){return Err("Junction arm is already connected to another road".into());}
        let(point,direction,def)=crate::junction_builder::port(project,arm)?;
        result.push((key,ed.splines.junction_plan(world,key,id,after.port(point,direction)?,&def)?));
    }Ok(result)
}

fn show_junction_connections(ed:&mut crate::editor::Editor,world:&crate::scene::World,id:i64,project:&crate::junction_builder::Project,parts:&[(crate::spline_editor::Key,omsi_map::MapSpline)],pending:Option<(crate::spline_editor::Key,usize)>)->Result<(),String>{
    let(_,at,heading,tilt,_,_)=ed.junction_placement(world,id)?;
    let pose=crate::junction_connections::Pose{at,heading,tilt};let local=|p:glam::DVec3|pose.local(p);
    let mut existing=Vec::new();let mut proposed=Vec::new();let mut links=[None;4];
    for(key,spline)in parts{
        let curve=omsi_geometry::SplineCurve::from_map(spline,glam::DVec2::new(key.0.0 as f64,key.0.1 as f64)*omsi_map::tile_size());
        let end=local(curve.point_at(if spline.prev_id==id{0.0}else{spline.length}));
        let arm=(0..4).filter_map(|i|crate::junction_builder::port(project,i).ok().map(|(p,_,_)|(i,p.distance(end)))).min_by(|a,b|a.1.total_cmp(&b.1)).filter(|(_,d)|*d<0.1).map(|(i,_)|i);
        let points=crate::spline_editor::SplineEditor::junction_preview(world,&[(*key,spline.clone())]).into_iter().map(|p|p.map(local));
        if pending.is_some_and(|(k,_)|k==*key){proposed.extend(points);}else{existing.extend(points);if let Some(i)=arm{links[i]=Some(key.1);}}
    }
    ed.junction_window.as_mut().unwrap().connection_preview(proposed,existing,links);Ok(())
}
