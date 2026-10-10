use crate::{app::App,traffic_editor::{Command,Window}};
use winit::keyboard::KeyCode;
impl App {
    pub(crate) fn editor_open_traffic_paths(&mut self) {
        if self.net.lan.is_some(){self.service_msg=Some(("KI-Pfade sind nur im Einzelspieler bearbeitbar".into(),5.0));return;}
        self.editor_terrain_finish();
        let(Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref())else{return;};
        let Some(key)=ed.splines.selected else {self.service_msg=Some(("Zuerst den Straßen- oder Terrain-Spline auswählen".into(),6.0));return;};
        ed.splines.finish_drag();ed.splines.cancel_connection();ed.splines.cancel_generation();ed.end_object_drag();
        ed.catalog=None;ed.sidewalk_window=None;ed.roadside_window=None;ed.junction_window=None;
        ed.tile_window=None;ed.text_window=None;ed.texture_target=None;ed.placing_asset=None;ed.object_stamp=None;
        ed.terrain.active=false;ed.spline_mode=true;ed.show_traffic_paths=true;ed.traffic_overlay.invalidate();world.collect_editor_splines();
        let mut w=Window::new(key);w.load_existing(world);w.refresh(world);ed.traffic_window=Some(w);
        self.menus.editor_drag=false;self.input.dragging=false;self.input.keys.clear();
        self.input.mouse_look=false;self.input.mmb_held=false;self.input.buttons_held=(false,false);self.release_vehicle_keys();
        if let Some(ui)=self.ui.as_mut(){ui.end_editor_dock_drag();}
        if let Some(window)=self.window.as_ref(){window.set_cursor_visible(true);}
    }
    pub(crate) fn editor_traffic_command(&mut self,command:Command){
        let _diagnostic = crate::editor_diagnostics::Span::new("ai_command", "");
        let(Some(world),Some(ed))=(self.world.clone(),self.menus.editor.as_mut())else{return;};
        if matches!(command,Command::Close){ed.traffic_window=None;self.input.keys.clear();return;}
        if matches!(command,Command::Overlay) {
            ed.show_traffic_paths=!ed.show_traffic_paths;ed.traffic_overlay.invalidate();return;
        }
        let Some(w)=ed.traffic_window.as_mut()else{return;};
        match command {
            Command::Free=>w.enable_free(),
            Command::Lane=>{w.active_lane=if w.settings.two_way{1-w.active_lane}else{0};w.ensure_nodes();},
            Command::ResetNode=>{if let Some(i)=w.selected_node {
                if let Some(start)=w.node_is_start(i) {let pi=w.settings.nodes[i].0;w.settings.connections.retain(|(lane,out,_)|*lane!=pi||*out==start);}
                w.settings.nodes[i].1[1..].fill(0.0);
            }},
            Command::NodeHeight(d)=>{if let Some(i)=w.selected_node {w.settings.nodes[i].1[3]=(w.settings.nodes[i].1[3]+d).clamp(-50.0,50.0);}},
            Command::Replace=>w.replace=!w.replace,
            Command::Connected=>w.settings.connected=!w.settings.connected,
            Command::TwoWay=>{w.settings.toggle_two_way();w.active_lane=0;w.selected_node=None;w.drag=None;},
            Command::Reverse=>{w.settings.reverse=!w.settings.reverse;w.settings.connections.clear();},
            Command::ForwardBus=>w.settings.forward_bus=!w.settings.forward_bus,
            Command::BackwardBus=>w.settings.backward_bus=!w.settings.backward_bus,
            Command::Width(d)=>w.settings.width=(w.settings.width+d).clamp(1.5,5.0),
            Command::Offset(d)=>w.settings.offset=(w.settings.offset+d).clamp(-20.0,20.0),
            Command::Height(d)=>w.settings.height=(w.settings.height+d).clamp(-5.0,5.0),
            Command::Apply=>{
                w.refresh(&world);if w.error.is_some(){return;}
                let Some(content)=crate::startup::content_dir() else {w.error=Some("Kein beschreibbarer Inhaltsordner".into());return;};
                if let Err(e)=crate::traffic_editor::write_definitions(&w.preview,&content,&self.args.root){w.error=Some(e);return;}
                let pieces=w.preview.pieces.clone();
                let replacing=w.replace;
                let result=if replacing {ed.splines.replace_traffic(&world,w.preview.replacements.clone())}else{ed.splines.apply_sidewalk(&world,pieces,None,false)};
                match result{
                    Ok(count)=>{ed.traffic_window=None;self.input.keys.clear();self.editor_reload_splines();
                        self.service_msg=Some((format!("KI-Pfade {}: {count} Abschnitte · Strg+Z rückgängig · Strg+S speichern · Karte neu laden für KI",if replacing {"ersetzt"}else{"angelegt"}),15.0));},
                    Err(e)=>ed.traffic_window.as_mut().unwrap().error=Some(e),
                }return;
            },Command::Refresh=>{},Command::Close|Command::Overlay=>{},
        }
        w.refresh(&world);
    }
    pub(crate) fn editor_traffic_input(&mut self,code:Option<KeyCode>,pressed:bool,repeat:bool)->bool{
        if self.menus.game_menu.is_some()||!self.menus.editor.as_ref().is_some_and(|e|e.traffic_window.is_some()){return false;}
        if let Some(k)=code {
            if matches!(k,KeyCode::KeyW|KeyCode::KeyA|KeyCode::KeyS|KeyCode::KeyD|KeyCode::KeyQ|KeyCode::KeyE|KeyCode::Space|KeyCode::ShiftLeft|KeyCode::ShiftRight){
                if pressed{self.input.keys.insert(k);}else{self.input.keys.remove(&k);}return true;
            }
            self.input.keys.remove(&k);
        }
        if pressed&&!repeat {match code {
            Some(KeyCode::Escape)=>self.editor_traffic_command(Command::Close),
            Some(KeyCode::Enter|KeyCode::NumpadEnter)=>self.editor_traffic_command(Command::Apply),_=>{}
        }}true
    }
    pub(crate) fn editor_traffic_mouse(&mut self,pressed:bool)->bool {
        let _diagnostic = crate::editor_diagnostics::Span::new("mouse_pick_snap", "");
        let Some(w)=self.menus.editor.as_ref().and_then(|e|e.traffic_window.as_ref())else{return false;};
        if !pressed{
            // Flush the latest queued position before snapping, even between redraws.
            self.editor_traffic_drag();
            let world=self.world.clone();
            if let Some(w)=self.menus.editor.as_mut().and_then(|e|e.traffic_window.as_mut()) {
                if let Some((index,_,_))=w.drag.take() {if let (Some(start),Some(world))=(w.node_is_start(index),world) {
                    if let Some((_,point))=w.preview.handles.iter().find(|(i,_)|*i==index) {
                        let mut hits:Vec<_>=w.targets.iter().filter(|t|t.start!=start&&t.point.distance(*point)<1.5).map(|t|(t.point.distance(*point),*t)).collect();
                        hits.sort_by(|a,b|a.0.total_cmp(&b.0));
                        if hits.len()>1 && (hits[1].0-hits[0].0).abs()<0.1 && (hits[1].1.key!=hits[0].1.key||hits[1].1.reversed!=hits[0].1.reversed) {
                            w.error=Some("Mehrere Anschlüsse liegen übereinander; keine eindeutige Verbindung gewählt".into());
                        } else if let Some((_,target))=hits.first() {w.snap_node(index,*target);w.refresh(&world);}
                    }
                }}
            }
            return true;
        }
        let command=w.hit(self.input.cursor);
        if self.ui.as_mut().is_some_and(|u|u.editor_hud_press(self.input.cursor)){return true;}
        if let Some(c)=command{self.editor_traffic_command(c);return true;}
        let w=self.menus.editor.as_ref().unwrap().traffic_window.as_ref().unwrap();
        if w.rect.is_some_and(|r|self.input.cursor.0>=r[0]&&self.input.cursor.0<=r[2]&&self.input.cursor.1>=r[1]&&self.input.cursor.1<=r[3])||!w.free{return true;}
        let (Some(cam),Some(surface))=(self.camera.as_ref(),self.gfx.surface.as_ref())else{return true;};
        let (origin,dir)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));let ray=dir.as_dvec3();
        let w=self.menus.editor.as_mut().unwrap().traffic_window.as_mut().unwrap();
        let hit=w.preview.handles.iter().filter(|(i,_)|w.settings.nodes[*i].0==w.active_lane).filter_map(|(i,p)|{
            let delta=*p+glam::DVec3::Z*0.4-origin;let t=delta.dot(ray);let distance=(delta-ray*t).length();
            (t>0.0&&distance<(t*0.006).clamp(0.35,1.5)).then_some((*i,*p,distance))
        }).min_by(|a,b|a.2.total_cmp(&b.2));
        if let Some((i,p,_))=hit {if ray.z.abs()>1e-5 {
            let t=(p.z-origin.z)/ray.z;
            if t>0.0 {w.selected_node=Some(i);w.drag=Some((i,p.z,p-(origin+ray*t)));}
        }}true
    }
    pub(crate) fn editor_traffic_queue_drag(&mut self) {
        if let Some(w)=self.menus.editor.as_mut().and_then(|e|e.traffic_window.as_mut()).filter(|w|w.drag.is_some()) {
            w.drag_pending=true;
            w.drag_events=w.drag_events.saturating_add(1);
            w.drag_queued_at.get_or_insert_with(std::time::Instant::now);
        }
    }
    pub(crate) fn editor_traffic_drag(&mut self) {
        let Some(w)=self.menus.editor.as_mut().and_then(|e|e.traffic_window.as_mut()) else{return;};
        if !std::mem::take(&mut w.drag_pending){return;}
        let events=std::mem::take(&mut w.drag_events);
        let queued_ms=w.drag_queued_at.take().map(|t|t.elapsed().as_millis()).unwrap_or(0);
        if queued_ms>=250 && std::env::var("OPENOMSI_EDITOR_DIAGNOSTICS").as_deref()==Ok("1") {
            log::warn!("EDITOR-DIAG drag_queue wait_ms={queued_ms} merged_events={events}");
        }
        let _diagnostic = crate::editor_diagnostics::Span::new("mouse_drag", format!("merged_events={events} queue_ms={queued_ms}"));
        if self.menus.game_menu.is_some()||!self.menus.editor.as_ref().is_some_and(|e|e.traffic_window.as_ref().is_some_and(|w|w.drag.is_some())){return;}
        let (Some(cam),Some(surface),Some(world))=(self.camera.as_ref(),self.gfx.surface.as_ref(),self.world.clone())else{return;};
        let (origin,dir)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));let ray=dir.as_dvec3();
        if ray.z.abs()<1e-5{return;}
        let w=self.menus.editor.as_mut().unwrap().traffic_window.as_mut().unwrap();let Some((index,z,offset))=w.drag else{return;};
        let t=(z-origin.z)/ray.z;if t<=0.0||t>2000.0{return;}
        w.move_node(index,origin+ray*t+offset);w.refresh(&world);
    }

}
