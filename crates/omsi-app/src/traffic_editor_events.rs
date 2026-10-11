use crate::{app::App,traffic_editor::{Command,Window}};
use winit::keyboard::KeyCode;
impl App {
    pub(crate) fn editor_open_path_manager(&mut self) {
        if self.menus.editor.as_ref().and_then(|e|e.traffic_window.as_ref()).is_some_and(connection_preview_pending) {
            self.service_msg=Some(("Connection is still a preview: add it, or press Ctrl+S to add and save. F9 keeps the curve.".into(),10.0));return;
        }
        if self.net.lan.is_some(){self.service_msg=Some(("AI paths can only be edited in single-player".into(),5.0));return;}
        self.editor_terrain_finish();
        let Some(ed)=self.menus.editor.as_mut()else{return;};
        ed.splines.finish_drag();ed.splines.cancel_connection();ed.splines.cancel_generation();ed.end_object_drag();
        ed.catalog=None;ed.sidewalk_window=None;ed.roadside_window=None;ed.junction_window=None;ed.tile_window=None;ed.text_window=None;ed.texture_target=None;ed.placing_asset=None;ed.object_stamp=None;ed.terrain.active=false;
        ed.show_traffic_paths=true;ed.traffic_overlay.invalidate();
        let mut w=Window::new(ed.splines.selected.unwrap_or(((0,0),0)));w.manager=Some(Default::default());ed.traffic_window=Some(w);
        self.menus.editor_drag=false;self.input.dragging=false;self.input.keys.clear();self.input.mouse_look=false;self.input.mmb_held=false;self.input.buttons_held=(false,false);self.release_vehicle_keys();
        if let Some(window)=self.window.as_ref(){window.set_cursor_visible(true);}
    }
    pub(crate) fn editor_open_traffic_paths(&mut self) {
        if self.net.lan.is_some(){self.service_msg=Some(("AI paths can only be edited in single-player".into(),5.0));return;}
        self.editor_terrain_finish();
        let(Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref())else{return;};
        let Some(key)=ed.splines.selected else {self.service_msg=Some(("Select the road or terrain spline first".into(),6.0));return;};
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
        self.editor_finish_path_drag();
        if matches!(command,Command::PathSave) {
            if self.menus.editor.as_ref().and_then(|e|e.traffic_window.as_ref()).is_some_and(connection_preview_pending) {
                self.editor_traffic_drag();
                let complete=self.menus.editor.as_ref().and_then(|e|e.traffic_window.as_ref()).and_then(|w|w.connector.as_ref()).is_some_and(|c|c.source.is_some()&&c.destination.is_some()&&c.picking.is_none());
                if !complete {self.service_msg=Some(("Connection is incomplete: select start and destination first.".into(),8.0));return;}
                self.editor_traffic_command(Command::Apply);
                // Applying closes the window only on success. Leave failed previews
                // editable, and never report them as saved map connections.
                if self.menus.editor.as_ref().is_some_and(|e|e.traffic_window.is_some()){return;}
            }
            self.editor_action(crate::editor::Action::Save);return;
        }
        let _diagnostic = crate::editor_diagnostics::Span::new("ai_command", "");
        let(Some(world),Some(ed))=(self.world.clone(),self.menus.editor.as_mut())else{return;};
        if matches!(command,Command::Close){ed.traffic_window=None;self.input.keys.clear();return;}
        if matches!(command,Command::Overlay) {
            ed.show_traffic_paths=!ed.show_traffic_paths;ed.traffic_overlay.invalidate();return;
        }
        if matches!(command,Command::Manage){self.editor_open_path_manager();return;}
        if matches!(command,Command::Generate){
            let chosen=ed.traffic_window.as_ref().and_then(|w|w.manager.as_ref()).and_then(|m|m.selected.as_ref()).filter(|l|l.source==1).and_then(|l|l.key);
            if let Some(key)=chosen {ed.splines.selected=Some((key.tile,key.id));}
            self.editor_open_traffic_paths();return;
        }
        let Some(w)=ed.traffic_window.as_mut()else{return;};
        if let Some(m)=w.manager.as_mut() {
            if matches!(command,Command::PathDelete|Command::PathEdit|Command::PathReset){if let Some(l)=m.selected.as_ref(){if let Some(k)=l.key {if let Err(e)=world.editor_lane_editable(k,l.source){w.error=Some(e);return;}}}}
            w.error=None;
            match command {
                Command::PathNext=>m.next(),Command::PathUndo=>ed.path_edits.undo(),Command::PathRedo=>ed.path_edits.redo(),
                Command::PathEdit=>{m.editing=!m.editing;},
                Command::PathDelete|Command::PathReset=>if let Some(l)=m.selected.as_ref() {
                    let mut c=ed.path_edits.current(l);
                    if matches!(command,Command::PathDelete){c.deleted=!c.deleted;m.editing=false;}else{c.shape=[[0.0;3];2];c.ends=[[0.0;3];2];c.floor=None;}
                    ed.path_edits.change(l,c);
                }, _=>{},
            }
            m.deleted=m.selected.as_ref().is_some_and(|l|ed.path_edits.current(l).deleted);
            m.can_undo=ed.path_edits.can_undo();m.can_redo=ed.path_edits.can_redo();return;
        }
        if matches!(command,Command::Connector) {
            if w.connector.is_some(){let start=w.start;*w=Window::new(start);w.load_existing(&world);}
            else {w.connector=Some(Default::default());w.replace=false;w.free=false;w.drag=None;w.selected_node=None;}
            w.refresh(&world);return;
        }
        if let Some(connector)=w.connector.as_mut() {
            match command {
                Command::ConnectorSource=>{connector.picking=Some(true);w.drag=None;},
                Command::ConnectorDestination=>{connector.picking=Some(false);w.drag=None;},
                Command::ConnectorReset=>{*connector=Default::default();w.drag=None;},
                Command::ForwardBus=>connector.bus_only=!connector.bus_only,
                Command::Width(d)=>connector.width=(connector.width+d).clamp(1.5,5.0),
                Command::Apply|Command::Refresh=>{},
                _=>return,
            }
            if !matches!(command,Command::Apply){w.refresh(&world);return;}
        }
        match command {
            Command::Manage|Command::PathDelete|Command::PathEdit|Command::PathReset|Command::PathNext|Command::PathUndo|Command::PathRedo|Command::PathSave|Command::Generate=>{},
            Command::Connector|Command::ConnectorSource|Command::ConnectorDestination|Command::ConnectorReset=>{},
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
                if let Some(c)=&w.connector {
                    if c.picking.is_some()||c.source.is_none()||c.destination.is_none(){return;}
                    for chosen in [c.source.unwrap(),c.destination.unwrap()] {
                        if !w.targets.iter().any(|t|t.key==chosen.key&&t.reversed==chosen.reversed&&t.start==chosen.start
                            &&t.point.distance(chosen.point)<0.1&&t.direction.dot(chosen.direction)>0.99) {
                            w.error=Some("Endpoint has changed or is not loaded. Select start and destination again.".into());return;
                        }
                    }
                    if w.preview.pieces.is_empty(){return;}
                    if crate::roadside_objects::has_additional_traffic_file(&world,&w.preview.pieces[0].1.file) {
                        w.error=Some("This additional connection already exists".into());return;
                    }
                    // Validate the actual bent path, not only its straight reference chord.
                    for (p,_) in &w.preview.markers {
                        let size=omsi_map::tile_size();
                        if world.tile_source((p.x/size).floor() as i32,(p.y/size).floor() as i32).is_none() {
                            w.error=Some("Connection curve leaves the existing map".into());return;
                        }
                    }
                }
                let Some(content)=crate::startup::content_dir() else {w.error=Some("Kein beschreibbarer Inhaltsordner".into());return;};
                if let Err(e)=crate::traffic_editor::write_definitions(&w.preview,&content,&self.args.root){w.error=Some(e);return;}
                let pieces=w.preview.pieces.clone();
                let replacing=w.replace;
                let result=if replacing {ed.splines.replace_traffic(&world,w.preview.replacements.clone())}else{ed.splines.apply_sidewalk(&world,pieces,None,false)};
                match result{
                    Ok(count)=>{ed.traffic_window=None;self.input.keys.clear();self.editor_reload_splines();
                        self.service_msg=Some((format!("AI paths {}: {count} sections · Ctrl+Z undo · Ctrl+S save · Reload map for AI traffic",if replacing {"replaced"}else{"created"}),15.0));},
                    Err(e)=>ed.traffic_window.as_mut().unwrap().error=Some(e),
                }return;
            },Command::Refresh=>{},Command::Close|Command::Overlay=>{},
        }
        w.refresh(&world);
    }
    pub(crate) fn editor_traffic_input(&mut self,code:Option<KeyCode>,pressed:bool,repeat:bool)->bool{
        if self.menus.game_menu.is_some()||!self.menus.editor.as_ref().is_some_and(|e|e.traffic_window.is_some()){return false;}
        if let Some(k)=code {
            let ctrl=self.input.keys.contains(&KeyCode::ControlLeft)||self.input.keys.contains(&KeyCode::ControlRight);
            let save=k==KeyCode::KeyS&&ctrl;
            if traffic_save_input(&mut self.input.keys,k,pressed) {
                if save&&pressed&&!repeat {self.editor_traffic_command(Command::PathSave);}
                return true;
            }
        }
        if pressed&&!repeat&&code==Some(KeyCode::F9){self.editor_finish_path_drag();self.editor_open_path_manager();return true;}
        if self.menus.editor.as_ref().is_some_and(|e|e.traffic_window.as_ref().is_some_and(|w|w.manager.is_some())) {
            let ctrl=self.input.keys.contains(&KeyCode::ControlLeft)||self.input.keys.contains(&KeyCode::ControlRight);
            if let Some(k)=code {
                if matches!(k,KeyCode::ControlLeft|KeyCode::ControlRight|KeyCode::ShiftLeft|KeyCode::ShiftRight|KeyCode::KeyW|KeyCode::KeyA|KeyCode::KeyS|KeyCode::KeyD|KeyCode::KeyQ|KeyCode::KeyE|KeyCode::Space) {
                    if pressed{self.input.keys.insert(k);}else{self.input.keys.remove(&k);}
                }
                if pressed&&!repeat {
                    let cmd=match k {KeyCode::Escape=>Some(Command::Close),KeyCode::Delete=>Some(Command::PathDelete),KeyCode::Tab=>Some(Command::PathNext),KeyCode::KeyS if ctrl=>Some(Command::PathSave),KeyCode::KeyZ if ctrl=>Some(Command::PathUndo),KeyCode::KeyY if ctrl=>Some(Command::PathRedo),_=>None};
                    if let Some(cmd)=cmd {self.editor_traffic_command(cmd);}
                }
            }return true;
        }
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
        if w.manager.is_some(){return self.editor_path_mouse(pressed);}
        if !pressed{
            // Flush the latest queued position before snapping, even between redraws.
            self.editor_traffic_drag();
            if let Some(w)=self.menus.editor.as_mut().and_then(|e|e.traffic_window.as_mut()).filter(|w|w.connector.is_some()) {
                w.drag=None;w.connector_drag=None;return true;
            }
            let world=self.world.clone();
            if let Some(w)=self.menus.editor.as_mut().and_then(|e|e.traffic_window.as_mut()) {
                if let Some((index,_,_))=w.drag.take() {if let (Some(start),Some(world))=(w.node_is_start(index),world) {
                    if let Some((_,point))=w.preview.handles.iter().find(|(i,_)|*i==index) {
                        let mut hits:Vec<_>=w.targets.iter().filter(|t|t.start!=start&&t.point.distance(*point)<1.5).map(|t|(t.point.distance(*point),*t)).collect();
                        hits.sort_by(|a,b|a.0.total_cmp(&b.0));
                        if hits.len()>1 && (hits[1].0-hits[0].0).abs()<0.1 && (hits[1].1.key!=hits[0].1.key||hits[1].1.reversed!=hits[0].1.reversed) {
                            w.error=Some("Several endpoints overlap; no unambiguous connection selected".into());
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
        if w.rect.is_some_and(|r|self.input.cursor.0>=r[0]&&self.input.cursor.0<=r[2]&&self.input.cursor.1>=r[1]&&self.input.cursor.1<=r[3])||(!w.free&&w.connector.is_none()){return true;}
        let (Some(cam),Some(surface))=(self.camera.as_ref(),self.gfx.surface.as_ref())else{return true;};
        let (origin,dir)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));let ray=dir.as_dvec3();
        let w=self.menus.editor.as_mut().unwrap().traffic_window.as_mut().unwrap();
        if let Some(c)=w.connector.as_mut() {
            if c.picking.is_some() {
                match c.pick(&w.targets,origin,ray) {
                    Ok(Some(t))=>{if let Err(error)=c.choose(t){w.error=Some(error);}else if let Some(world)=self.world.as_ref(){w.refresh(world);}},
                    Ok(None)=>w.error=Some("Click a marked endpoint; arrows show travel direction".into()),
                    Err(error)=>w.error=Some(error),
                }
            } else if let Some((i,p))=w.preview.handles.iter().filter_map(|(i,p)|{
                let delta=*p+glam::DVec3::Z*0.4-origin;let t=delta.dot(ray);
                (t>0.0&&(delta-ray*t).length()<(t*0.006).clamp(0.35,1.5)).then_some((*i,*p))
            }).next() {
                let p=p+glam::DVec3::Z*0.4;let t=(p-origin).dot(ray);
                w.selected_node=Some(i);w.connector_drag=Some(crate::editor_path_edit::PathDrag {index:i,point:p,normal:ray,offset:p-(origin+ray*t),before:Default::default()});
            }
            return true;
        }
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
        if let Some(drag)=self.menus.editor.as_ref().and_then(|e|e.traffic_window.as_ref()).and_then(|w|w.connector_drag) {
            if self.menus.game_menu.is_some(){return;}
            if let (Some(cam),Some(surface),Some(world))=(self.camera.as_ref(),self.gfx.surface.as_ref(),self.world.as_ref()) {
                let (origin,ray)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));
                if let Some(mut p)=drag.target(origin,ray.as_dvec3()) {
                    if let Some(z)=world.editor_road_height(p){p.z=z;}
                    if let Some(w)=self.menus.editor.as_mut().and_then(|e|e.traffic_window.as_mut()) {
                        if let Some(c)=w.connector.as_mut(){c.move_handle(drag.index,p);}w.refresh(world);
                    }
                }
            }return;
        }
        if self.menus.editor.as_ref().is_some_and(|e|e.traffic_window.as_ref().is_some_and(|w|w.manager.is_some())) {self.editor_path_drag();return;}
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
        if let Some(c)=w.connector.as_mut(){c.move_handle(index,origin+ray*t+offset);}
        else {w.move_node(index,origin+ray*t+offset);}
        w.refresh(&world);
    }

}

impl App {
    fn editor_finish_path_drag(&mut self) {
        self.editor_path_drag();
        if let Some(ed)=self.menus.editor.as_mut(){if let Some(m)=ed.traffic_window.as_mut().and_then(|w|w.manager.as_mut()) {
            if let (Some(drag),Some(l))=(m.drag.take(),m.selected.as_ref()) {let after=ed.path_edits.current(l);ed.path_edits.commit_drag(l,drag.before,after);}
        }}
    }
    fn editor_path_mouse(&mut self,pressed:bool)->bool {
        if !pressed {
            self.editor_path_drag();
            let world=self.world.clone();
            if let Some(ed)=self.menus.editor.as_mut() {
                let snap_targets=ed.traffic_window.as_ref().map(|w|w.targets.clone()).unwrap_or_default();
                if let Some(m)=ed.traffic_window.as_mut().and_then(|w|w.manager.as_mut()) {
                    if let (Some(drag),Some(l))=(m.drag.take(),m.selected.as_ref()) {
                        if drag.index>=2 {
                            if let Some(view)=ed.path_edits.preview(l) {
                                let p=if drag.index==2{view.start()}else{view.end()};
                                let mut hits:Vec<_>=snap_targets.iter().filter(|t|t.start==(drag.index==3)&&l.key!=Some(t.key)&&t.point.distance(p)<1.0).copied().collect();
                                hits.sort_by(|a,b|a.point.distance(p).total_cmp(&b.point.distance(p)));
                                if let Some(t)=hits.first().filter(|t|hits.get(1).is_none_or(|other|other.point.distance(p)-t.point.distance(p)>0.1)) {
                                    let mut base=l.clone();base.editor_path_ends=[[0.0;3];2];base.editor_path_floor=None;crate::editor_path_edit::apply_shape(&mut base,[[0.0;3];2]);
                                    let mut c=ed.path_edits.current(l);c.ends[drag.index-2]=(t.point-if drag.index==2{base.start()}else{base.end()}).to_array();Self::editor_project_path_floor(world.as_deref(),l,&mut c);ed.path_edits.drag(l,c);
                                }
                            }
                        }
                        let after=ed.path_edits.current(l);
                        // One drag is one undo step, not one step per frame.
                        ed.path_edits.commit_drag(l,drag.before,after);
                    }
                }
            }return true;
        }
        let w=self.menus.editor.as_ref().unwrap().traffic_window.as_ref().unwrap();
        if let Some(command)=w.hit(self.input.cursor){self.editor_traffic_command(command);return true;}
        if self.ui.as_mut().is_some_and(|u|u.editor_hud_press(self.input.cursor)){return true;}
        let w=self.menus.editor.as_ref().unwrap().traffic_window.as_ref().unwrap();
        if w.rect.is_some_and(|r|self.input.cursor.0>=r[0]&&self.input.cursor.0<=r[2]&&self.input.cursor.1>=r[1]&&self.input.cursor.1<=r[3]){return true;}
        let(Some(cam),Some(surface))=(self.camera.as_ref(),self.gfx.surface.as_ref())else{return true;};
        let(origin,ray)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));let ray=ray.as_dvec3();
        let ed=self.menus.editor.as_mut().unwrap();let m=ed.traffic_window.as_mut().unwrap().manager.as_mut().unwrap();
        if m.editing {
            if let Some(l)=m.selected.as_ref().and_then(|l|ed.path_edits.preview(l)) {
                for (i,p) in crate::editor_path_edit::Manager::handles(&l) {
                    let p=p+glam::DVec3::Z*0.4;let d=p-origin;let t=d.dot(ray);
                    if t>0.0&&(d-ray*t).length()<(t*0.006).clamp(0.4,1.5) {
                        let hit=origin+ray*t;
                        m.drag=Some(crate::editor_path_edit::PathDrag {index:i,point:p,normal:ray,offset:p-hit,before:ed.path_edits.current(&l)});
                        log::info!("AI path drag started: {:?}, handle {}",crate::editor_path_edit::Id::of(&l),i+1);return true;
                    }
                }
            }
        }
        m.pick(origin,ray);
        if let Some(l)=m.selected.as_ref(){log::info!("AI path selected: {:?}, {} points, curve editing {}",crate::editor_path_edit::Id::of(l),l.points.len(),m.editing);}
        if let (Some(world),Some(l))=(self.world.as_ref(),m.selected.as_ref()) {if let Some(k)=l.key {if let Err(e)=world.editor_lane_editable(k,l.source){m.selected=None;self.service_msg=Some((e,10.0));return true;}}}
        if let Some(l)=m.selected.as_ref() {if l.source==1 {if let Some(k)=l.key{ed.splines.selected=Some((k.tile,k.id));}}}
        true
    }
    fn editor_project_path_floor(world:Option<&crate::scene::World>,l:&omsi_sim::traffic::Lane,c:&mut crate::editor_path_edit::Change) {
        let Some(world)=world else{return;};
        let mut view=l.clone();view.editor_path_floor=None;view.editor_path_ends=c.ends;crate::editor_path_edit::apply_shape(&mut view,c.shape);
        let mut heights=[0.0;65];
        for (i,z) in heights.iter_mut().enumerate() {
            let at=(view.points.len()-1) as f64*i as f64/64.0;let j=(at.floor() as usize).min(view.points.len()-2);let p=view.points[j].lerp(view.points[j+1],at-j as f64);
            if i==0||i==64 {*z=p.z;}else{let Some(h)=world.editor_road_height(p)else{c.floor=None;return;};*z=h;}
        }
        c.floor=Some(heights);
    }
    fn editor_path_drag(&mut self) {
        if self.menus.game_menu.is_some(){return;}
        let Some(drag)=self.menus.editor.as_ref().and_then(|e|e.traffic_window.as_ref()).and_then(|w|w.manager.as_ref()).and_then(|m|m.drag)else{return;};
        let(Some(cam),Some(surface))=(self.camera.as_ref(),self.gfx.surface.as_ref())else{return;};
        let(origin,ray)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));let ray=ray.as_dvec3();
        let Some(mut target)=drag.target(origin,ray)else{return;};
        if let Some(z)=self.world.as_ref().and_then(|w|w.editor_road_height(target)){target.z=z;}
        let (index,before)=(drag.index,drag.before);let world=self.world.clone();
        let ed=self.menus.editor.as_mut().unwrap();let m=ed.traffic_window.as_mut().unwrap().manager.as_mut().unwrap();
        let Some(l)=m.selected.as_ref()else{return;};
        let mut base=l.clone();base.editor_path_ends=[[0.0;3];2];base.editor_path_floor=None;crate::editor_path_edit::apply_shape(&mut base,[[0.0;3];2]);
        if index>=2 {
            let mut c=before;let original=if index==2{base.start()}else{base.end()};
            c.ends[index-2]=(target-original).clamp(glam::DVec3::splat(-200.0),glam::DVec3::splat(200.0)).to_array();Self::editor_project_path_floor(world.as_deref(),l,&mut c);ed.path_edits.drag(l,c);return;
        }
        let (p,h)=crate::editor_path_edit::Manager::control(&base,index);
        let direction=omsi_geometry::SplineCurve::dir(h).extend(0.0);let side=glam::DVec3::new(direction.y,-direction.x,0.0);
        let t=(index+1) as f64/3.0;let endpoint_offset=glam::DVec3::from_array(before.ends[0])*(1.0-t)+glam::DVec3::from_array(before.ends[1])*t;
        let delta=(target-p-endpoint_offset)/ (16.0*(1.0_f64/3.0).powi(2)*(2.0_f64/3.0).powi(2));
        let mut c=before;c.shape[index]=[delta.dot(side).clamp(-50.0,50.0),delta.dot(direction).clamp(-50.0,50.0),before.shape[index][2]];
        Self::editor_project_path_floor(world.as_deref(),l,&mut c);ed.path_edits.drag(l,c);
    }
}


// All AI-path windows intercept input before normal editor shortcuts. Preserve both
// Control keys, and consume save before S can become a held camera movement key.
fn traffic_save_input(keys:&mut hashbrown::HashSet<KeyCode>,code:KeyCode,pressed:bool)->bool {
    if matches!(code,KeyCode::ControlLeft|KeyCode::ControlRight) {
        if pressed {keys.insert(code);keys.remove(&KeyCode::KeyS);}else{keys.remove(&code);}
        return true;
    }
    if code==KeyCode::KeyS&&(keys.contains(&KeyCode::ControlLeft)||keys.contains(&KeyCode::ControlRight)) {
        keys.remove(&KeyCode::KeyS);return true;
    }
    false
}
#[cfg(test)]
mod save_shortcut_tests {
    use super::*;
    #[test]
    fn either_control_key_saves_without_held_camera_movement() {
        for ctrl in [KeyCode::ControlLeft,KeyCode::ControlRight] {
            let mut keys=hashbrown::HashSet::new();
            keys.insert(KeyCode::KeyS);
            assert!(traffic_save_input(&mut keys,ctrl,true));assert!(!keys.contains(&KeyCode::KeyS));
            assert!(traffic_save_input(&mut keys,KeyCode::KeyS,true));assert!(!keys.contains(&KeyCode::KeyS));
            assert!(traffic_save_input(&mut keys,KeyCode::KeyS,true)); // OS repeat remains consumed.
            assert!(traffic_save_input(&mut keys,KeyCode::KeyS,false));
            assert!(traffic_save_input(&mut keys,ctrl,false));assert!(keys.is_empty());
            assert!(!traffic_save_input(&mut keys,KeyCode::KeyS,true)); // Plain S still moves the camera.
        }
    }
}

fn connection_preview_pending(w:&Window)->bool {
    w.connector.as_ref().is_some_and(|c|c.source.is_some()||c.destination.is_some())
}
