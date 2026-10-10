use crate::{app::App,sidewalk::{Command,Window}};
use winit::keyboard::KeyCode;
impl App {
    pub(crate) fn editor_open_sidewalk(&mut self) {
        if self.net.lan.is_some(){self.service_msg=Some(("Sidewalk editing is available in single-player".into(),5.0));return;}
        self.editor_terrain_finish();let(Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref())else{return;};
        ed.splines.finish_drag();ed.splines.cancel_connection();ed.splines.cancel_generation();ed.end_object_drag();
        ed.catalog=None;ed.roadside_window=None;ed.junction_window=None;ed.tile_window=None;ed.text_window=None;ed.texture_target=None;ed.placing_asset=None;ed.object_stamp=None;ed.terrain.active=false;ed.spline_mode=true;
        world.collect_editor_splines();let mut w=Window::new(ed.splines.selected);
        if let Some((key,s))=ed.splines.selected.and_then(|k|world.spline_edits.lock().current(k).map(|s|(k,s))){
            if world.spline_type(&s.file).is_some_and(|t|t.def.paths.iter().any(|p|p.kind==0)){w.settings.length=s.length.min(20.0);}else{
                w.existing=Some(key);w.file=s.file;w.settings.mirror=s.mirror;w.settings.length=s.length;w.start=None;w.picking=1;w.message="Existing sidewalk selected – now click the reference road".into();
            }}
        w.can_undo=ed.splines.can_undo();w.refresh(world);ed.sidewalk_window=Some(w);self.menus.editor_drag=false;self.input.dragging=false;self.input.keys.clear();self.input.mouse_look=false;self.input.mmb_held=false;self.input.buttons_held=(false,false);self.release_vehicle_keys();
        if let Some(ui)=self.ui.as_mut(){ui.end_editor_dock_drag();}if let Some(window)=self.window.as_ref(){window.set_cursor_visible(true);}
    }
    pub(crate) fn editor_sidewalk_command(&mut self,command:Command){
        let(Some(world),Some(ed))=(self.world.clone(),self.menus.editor.as_mut())else{return;};let Some(w)=ed.sidewalk_window.as_mut()else{return;};
        if matches!(command,Command::Close){ed.sidewalk_window=None;ed.traffic_window=None;self.input.keys.clear();return;}
        if !w.commit(){return;}
        match command {
            Command::Catalog=>{ed.catalog=Some(crate::asset_catalog::Catalog::with_map(world.root.clone(),crate::asset_catalog::Kind::Spline,Some(world.map_dir.clone())));self.input.keys.clear();return;},
            Command::Existing=>{w.picking=2;w.message="Click existing sidewalk; profile and textures will be used".into();return;},
            Command::PickRoad=>{w.picking=1;w.message="Click reference road".into();return;},
            Command::New=>{w.existing=None;w.message="Create new sidewalk; existing splines are kept".into();},
            Command::Edit(f)=>{w.edit(f);self.input.keys.clear();return;},
            Command::Adjust(f,d)=>{if let Err(e)=w.set(f,w.value(f)+d){w.message=e;return;}},
            Command::Sides(side)=>w.settings.sides=side,Command::Connected(v)=>{w.settings.connected=v;w.settings.start=0.0;},Command::Mirror=>w.settings.mirror=!w.settings.mirror,Command::Detach=>w.settings.detach=!w.settings.detach,
            Command::Apply=>{
                w.refresh(&world);if let Some(e)=&w.error{w.message=e.clone();return;}
                let existing=w.existing;let detach=w.settings.detach;let pieces=w.preview.pieces.clone();
                match ed.splines.apply_sidewalk(&world,pieces,existing,detach){Ok(count)=>{ed.sidewalk_window=None;ed.traffic_window=None;self.input.keys.clear();self.editor_reload_splines();self.service_msg=Some((format!("Sidewalk applied: {count} sections · Ctrl+Z to undo · Ctrl+S to save"),10.0));},Err(e)=>{ed.sidewalk_window.as_mut().unwrap().message=e;}}return;
            },
            Command::Undo=>{w.message=ed.splines.undo(&world);w.can_undo=ed.splines.can_undo();w.refresh(&world);self.editor_reload_splines();return;},
            Command::Save=>{self.editor_action(crate::editor::Action::Save);return;},Command::Preview=>{},Command::Close=>{}
        }w.refresh(&world);
    }
    pub(crate) fn editor_sidewalk_input(&mut self,code:Option<KeyCode>,text:Option<&str>,pressed:bool,repeat:bool)->bool{
        if self.menus.game_menu.is_some()||!self.menus.editor.as_ref().is_some_and(|e|e.sidewalk_window.is_some()&&e.catalog.is_none()){return false;}
        let editing=self.menus.editor.as_ref().unwrap().sidewalk_window.as_ref().unwrap().input.is_some();
        if repeat&&!editing&&matches!(code,Some(KeyCode::Enter|KeyCode::NumpadEnter|KeyCode::Escape|KeyCode::KeyZ|KeyCode::KeyS)){return true;}
        let ctrl=self.input.keys.contains(&KeyCode::ControlLeft)||self.input.keys.contains(&KeyCode::ControlRight);
        if let Some(k)=code{
            if matches!(k,KeyCode::ControlLeft|KeyCode::ControlRight|KeyCode::ShiftLeft|KeyCode::ShiftRight)||(!editing&&!ctrl&&matches!(k,KeyCode::KeyW|KeyCode::KeyA|KeyCode::KeyS|KeyCode::KeyD|KeyCode::KeyQ|KeyCode::KeyE|KeyCode::Space)){
                if pressed{self.input.keys.insert(k);}else{self.input.keys.remove(&k);}return true;
            }self.input.keys.remove(&k);
        }if !pressed{return true;}
        if !editing{let command=match code{Some(KeyCode::Escape)=>Some(Command::Close),Some(KeyCode::Enter|KeyCode::NumpadEnter)=>Some(Command::Apply),Some(KeyCode::KeyZ)if ctrl=>Some(Command::Undo),Some(KeyCode::KeyS)if ctrl=>Some(Command::Save),_=>None};if let Some(c)=command{self.editor_sidewalk_command(c);}return true;}
        let w=self.menus.editor.as_mut().unwrap().sidewalk_window.as_mut().unwrap();
        match code{
            Some(KeyCode::Escape)=>{w.input=None;},Some(KeyCode::Enter|KeyCode::NumpadEnter)=>{self.editor_sidewalk_command(Command::Preview);},
            Some(KeyCode::KeyA)if ctrl=>w.input.as_mut().unwrap().replace=true,
            Some(KeyCode::Backspace)=>{let i=w.input.as_mut().unwrap();if i.replace{i.text.clear();}else{i.text.pop();}i.replace=false;},
            Some(KeyCode::Delete)=>{let i=w.input.as_mut().unwrap();i.text.clear();i.replace=false;},
            _ if !ctrl=>if let Some(text)=text{let t:String=text.chars().filter(|c|c.is_ascii_digit()||matches!(c,'-'|'+'|','|'.')).collect();if !t.is_empty(){let i=w.input.as_mut().unwrap();if i.replace{i.text.clear();i.replace=false;}if i.text.len()+t.len()<=24{i.text.push_str(&t);}}},_=>{}
        }true
    }
    pub(crate) fn editor_sidewalk_mouse(&mut self,pressed:bool)->bool{
        if !self.menus.editor.as_ref().is_some_and(|e|e.sidewalk_window.is_some()&&e.catalog.is_none()){return false;}
        if !pressed{self.menus.editor.as_mut().unwrap().sidewalk_window.as_mut().unwrap().drag=None;return true;}
        if self.ui.as_mut().is_some_and(|u|u.editor_hud_press(self.input.cursor)){return true;}
        let w=self.menus.editor.as_ref().unwrap().sidewalk_window.as_ref().unwrap();if let Some(c)=w.hit(self.input.cursor){self.editor_sidewalk_command(c);return true;}if w.contains(self.input.cursor){return true;}
        let(Some(cam),Some(surface),Some(world))=(self.camera.as_ref(),self.gfx.surface.as_ref(),self.world.clone())else{return true;};
        let(origin,direction)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));let ray=direction.as_dvec3();
        let ed=self.menus.editor.as_mut().unwrap();let w=ed.sidewalk_window.as_mut().unwrap();if !w.commit(){return true;}
        if w.picking==0{
            let hit=w.preview.handles.iter().enumerate().filter_map(|(i,p)|{let d=*p+glam::DVec3::Z*0.4-origin;let t=d.dot(ray);let distance=(d-ray*t).length();(t>0.0&&distance<1.0).then_some((i,distance))}).min_by(|a,b|a.1.total_cmp(&b.1));
            if let Some((i,_))=hit{w.drag=Some(i==0);return true;}
            w.message="Drag start or end marker; to switch, click Choose road/sidewalk first".into();return true;
        }
        ed.splines.pick(&world,origin,direction);let Some(key)=ed.splines.selected else{return true;};let Some(s)=world.spline_edits.lock().current(key)else{return true;};
        let w=ed.sidewalk_window.as_mut().unwrap();if w.picking==1 {
            if !world.spline_type(&s.file).is_some_and(|t|t.def.paths.iter().any(|p|p.kind==0)){w.message="Select a road with driving paths".into();return true;}
            w.start=Some(key);w.settings.start=0.0;w.settings.length=w.settings.length.min(s.length).max(0.5);
        }else{if Some(key)==w.start{w.message="Sidewalk and road must differ".into();return true;}w.existing=Some(key);w.file=s.file;w.settings.mirror=s.mirror^(w.settings.sides==1);w.settings.length=s.length.min(w.preview.total.max(0.5));}
        w.picking=0;w.message="Check preview; profile and textures are kept".into();w.refresh(&world);self.menus.editor_drag=false;true
    }
    pub(crate) fn editor_sidewalk_drag(&mut self){
        if self.menus.game_menu.is_some()||!self.menus.editor.as_ref().is_some_and(|e|e.catalog.is_none()&&e.sidewalk_window.as_ref().is_some_and(|w|w.drag.is_some())){return;}
        let(Some(cam),Some(surface),Some(world))=(self.camera.as_ref(),self.gfx.surface.as_ref(),self.world.clone())else{return;};let(origin,dir)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));
        let Some(point)=crate::placing::ground_hit(&world,origin,dir.as_dvec3(),2000.0)else{return;};
        let w=self.menus.editor.as_mut().unwrap().sidewalk_window.as_mut().unwrap();let Some(station)=crate::sidewalk::project_station(&w.route,point)else{return;};
        w.drag_to(station);w.refresh(&world);
    }
}
