//! The object editor (`crate::editor`) as the app works it: on and off, keys, mouse, wheel.

use super::*;

impl App {
    pub(crate) fn remember_editor_camera(&self) {
        if self.menus.editor.is_none() || self.world.is_none() { return; }
        let Some(c) = self.camera.as_ref() else { return };
        let key = omsi_launcher_lib::editor_views::key(&self.args.root, &self.args.map);
        let view = omsi_launcher_lib::editor_views::View {
            position: [c.position.x, c.position.y, c.position.z],
            yaw: c.yaw, pitch: c.pitch, roll: c.roll, fov: c.fov_deg,
        };
        let dir = omsi_launcher_lib::data_dir().join("editor-views");
        if let Err(e) = omsi_launcher_lib::editor_views::save(&dir, &key, view) {
            log::warn!("Could not save editor camera position: {e}");
        }
    }

    /// A key while the game menu is open.
    /// The object editor on or off; on, it starts with the free camera where the view is.
    pub(crate) fn toggle_editor(&mut self) {
        self.remember_editor_camera();
        self.editor_terrain_finish();
        // (in a LAN session the host edits the map for everybody: its edits go to the
        // others' games, a client's would stay its own)
        if self.menus.editor.is_none() && self.net.lan.as_ref().map(|l| l.role == omsi_net::Role::Client).unwrap_or(false) {
            self.service_msg = Some(("In a LAN session only the host edits the map".into(), 3.0));
            return;
        }
        if !crate::editor::toggle_session(&mut self.menus.editor, &mut self.menus.editor_paused) {
            self.menus.editor_drag = false;
            self.editor_reload_splines();
            self.service_msg = Some(("Object editor off (unsaved changes stay until the end of the session)".into(), 3.0));
            return;
        }
        self.service_msg = None;
    }

    /// A key while the object editor is on; true when it was the editor's.
    pub(crate) fn editor_key(&mut self, code: KeyCode) -> bool {
        if self.menus.editor.as_ref().is_some_and(|ed|ed.terrain.active) {
            use crate::terrain_editor::{Command,Field};
            let shift=self.input.keys.contains(&KeyCode::ShiftLeft)||self.input.keys.contains(&KeyCode::ShiftRight);
            let ctrl=self.input.keys.contains(&KeyCode::ControlLeft)||self.input.keys.contains(&KeyCode::ControlRight);
            let command=match code {
                KeyCode::Escape=>Some(Command::Exit),
                KeyCode::KeyZ if ctrl=>Some(if shift {Command::Redo} else {Command::Undo}),
                KeyCode::KeyY if ctrl=>Some(Command::Redo),
                KeyCode::KeyS if ctrl=>Some(Command::Save),
                KeyCode::BracketLeft=>Some(Command::Adjust(Field::Radius,-1.0)),
                KeyCode::BracketRight=>Some(Command::Adjust(Field::Radius,1.0)),
                _=>None,
            };
            if let Some(command)=command {self.editor_terrain_command(command);return true;}
            // Camera letters have no editor binding; all object/spline keys stay inactive.
            return crate::editor::action_for(code,shift,ctrl,0.0).is_some();
        }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.placing_asset.is_some()) {
            if matches!(code, KeyCode::Escape | KeyCode::KeyB) {
                let ed = self.menus.editor.as_mut().unwrap(); ed.placing_asset = None;ed.object_stamp=None;ed.splines.cancel_generation();
                self.service_msg = Some(("Placement finished; placed objects are kept · Ctrl+S to save".into(), 4.0));
                return true;
            }
            let object=self.menus.editor.as_ref().is_some_and(|ed|ed.placing_asset.as_ref().is_some_and(|a|a.kind==crate::asset_catalog::Kind::Object));
            let ctrl=self.input.keys.contains(&KeyCode::ControlLeft)||self.input.keys.contains(&KeyCode::ControlRight);
            if matches!(code,KeyCode::KeyW|KeyCode::KeyA|KeyCode::KeyS|KeyCode::KeyD|KeyCode::KeyQ|KeyCode::KeyE) && !(ctrl&&code==KeyCode::KeyS) {return false;}
            if object {
                let fine=self.input.keys.contains(&KeyCode::ShiftLeft)||self.input.keys.contains(&KeyCode::ShiftRight);
                if let Some(step)=crate::object_angles::key(code,fine,ctrl) {
                    let ed=self.menus.editor.as_mut().unwrap();
                    if let Some(stamp)=ed.object_stamp.as_mut() {
                        match step {
                            crate::object_angles::Step::Turn(t)=>stamp.heading+=t,
                            crate::object_angles::Step::Tilt(axis,d)=>{
                                if !crate::editor::builder_asset(&stamp.sco) {self.service_msg=Some(("Tilt controls currently support builder junctions and roundabouts".into(),5.0));return true;}
                                match crate::object_angles::adjusted(stamp.tilt,axis,d) {Ok(tilt)=>stamp.tilt=tilt,Err(e)=>{self.service_msg=Some((e,5.0));return true;}}
                            }
                        }
                        ed.align_object=false;
                    }
                    return true;
                }
            }
            if object && matches!(code,KeyCode::KeyN|KeyCode::KeyM|KeyCode::KeyU|KeyCode::KeyO) {
                let fine=self.input.keys.contains(&KeyCode::ShiftLeft)||self.input.keys.contains(&KeyCode::ShiftRight);
                let ed=self.menus.editor.as_mut().unwrap();
                if let Some(stamp)=ed.object_stamp.as_mut() {
                    match code {KeyCode::KeyN=>stamp.heading-=if fine {0.5}else{5.0},KeyCode::KeyM=>stamp.heading+=if fine {0.5}else{5.0},
                        KeyCode::KeyU=>stamp.height_offset-=if fine {0.05}else{0.5},_=>stamp.height_offset+=if fine {0.05}else{0.5}}
                    ed.align_object=false;
                }
                return true;
            }
            if object && code==KeyCode::Delete {let ed=self.menus.editor.as_mut().unwrap();ed.placing_asset=None;ed.object_stamp=None;}
            else if !matches!(code,KeyCode::KeyP|KeyCode::KeyX) && !(ctrl&&matches!(code,KeyCode::KeyC|KeyCode::KeyV|KeyCode::KeyR|KeyCode::KeyS)) {return true;}
        }
        let shift = self.input.keys.contains(&KeyCode::ShiftLeft) || self.input.keys.contains(&KeyCode::ShiftRight);
        let ctrl = self.input.keys.contains(&KeyCode::ControlLeft) || self.input.keys.contains(&KeyCode::ControlRight);
        let Some(cam) = self.camera.as_ref() else { return false };
        let Some(action) = crate::editor::action_for_mode(code, shift, ctrl, cam.yaw as f64,self.menus.editor.as_ref().is_some_and(|ed|ed.spline_mode)) else { return false };
        self.editor_action(action)
    }

    /// Mouse buttons and keys run the same editor action and validation.
    pub(crate) fn editor_action(&mut self, action: crate::editor::Action) -> bool {
        if self.menus.editor.is_none() { return false; }
        if matches!(action,crate::editor::Action::AuditSplines){self.editor_open_audit();return true;}
        if matches!(action,crate::editor::Action::ReloadMap){self.editor_reload_map();return true;}
        let Some(cam) = self.camera.as_ref() else { return false; };
        let (eye, fwd) = (cam.position, cam.forward());
        let Some(world) = self.world.clone() else { return false };
        if matches!(action,crate::editor::Action::Copy|crate::editor::Action::ClipboardCopy|crate::editor::Action::Paste|crate::editor::Action::RepeatObject) {
            self.menus.editor_drag=false;
        }
        if matches!(action,crate::editor::Action::Save|crate::editor::Action::Leave|crate::editor::Action::SplineMode
            |crate::editor::Action::TileWindow|crate::editor::Action::TerrainMode|crate::editor::Action::Catalog|crate::editor::Action::JunctionWindow|crate::editor::Action::RoundaboutWindow|crate::editor::Action::RoadsideWindow
            |crate::editor::Action::Paste|crate::editor::Action::RepeatObject) {
            self.editor_terrain_finish();
        }
        if matches!(action, crate::editor::Action::FitTerrain | crate::editor::Action::SmoothRoad(_)) { self.menus.editor_drag = false; }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.spline_mode && ed.splines.connection_active())
            && !matches!(&action, crate::editor::Action::Connect | crate::editor::Action::ReplaceConnection | crate::editor::Action::Generate
                | crate::editor::Action::SnapEnd | crate::editor::Action::Pick | crate::editor::Action::NextPick
                | crate::editor::Action::SplineUndo | crate::editor::Action::Straight | crate::editor::Action::Leave
                | crate::editor::Action::SplineMode | crate::editor::Action::Save | crate::editor::Action::TileWindow | crate::editor::Action::TerrainMode | crate::editor::Action::JunctionWindow | crate::editor::Action::RoundaboutWindow | crate::editor::Action::RoadsideWindow)
        {
            self.service_msg = Some(("Choose Connect or Cancel first".into(), 4.0));
            return true;
        }
        let msg = match action {
            crate::editor::Action::ObjectText => {
                if self.net.lan.is_some() { "Object text editing is currently available in single-player".into() }
                else {
                    self.editor_terrain_finish(); self.menus.editor_drag = false; self.input.mouse_look = false;
                    match self.menus.editor.as_mut().unwrap().open_labels(&world) {
                        Ok(()) => { self.input.keys.clear(); self.input.buttons_held = (false,false); self.input.dragging = false;
                            self.input.mmb_held = false; self.input.cursor_hidden = None;
                            if let Some(window) = self.window.as_ref() { window.set_cursor_visible(true); }
                            "Click text field and enter text".into() }
                        Err(error) => error,
                    }
                }
            }
            crate::editor::Action::CancelPlacement=>{let ed=self.menus.editor.as_mut().unwrap();ed.placing_asset=None;ed.object_stamp=None;ed.splines.cancel_generation();"Placement finished · Ctrl+S to save".into()}
            crate::editor::Action::PlacementRepeat=>{let ed=self.menus.editor.as_mut().unwrap();ed.repeat_objects=!ed.repeat_objects;
                if ed.repeat_objects {"Repeat placement ON · Click each target · Esc to finish"} else {"Place once"}.into()}
            crate::editor::Action::ClipboardCopy=>{let ed=self.menus.editor.as_mut().unwrap();
                if ed.spline_mode {"Switch to object mode before copying an object".into()} else {
                    match ed.capture_object(&world) {Ok(stamp)=>{ed.clipboard=Some(stamp);"Object copied · Ctrl+V and click target".into()},Err(e)=>e}
                }}
            crate::editor::Action::Paste|crate::editor::Action::RepeatObject=>{
                let ed=self.menus.editor.as_mut().unwrap();let stamp=if matches!(action,crate::editor::Action::Paste) {ed.clipboard.clone()} else {ed.last_object.clone()};
                match stamp {Some(stamp)=>{self.menus.editor_drag=false;ed.start_object(stamp);"Click target · Repeat placement for more copies · Esc to finish".into()},None=>"No object stored for placement yet".into()}
            }
            crate::editor::Action::TerrainMode=>{self.editor_open_terrain();return true;}
            crate::editor::Action::SidewalkWindow=>{self.editor_open_sidewalk();return true;}
            crate::editor::Action::RoadsideWindow=>{self.editor_open_roadside();return true;}
            crate::editor::Action::RoundaboutWindow=>{self.editor_open_roundabout();return true;}
            crate::editor::Action::JunctionWindow=>{self.editor_open_junction();return true;}
            crate::editor::Action::TileWindow => { self.editor_open_tiles(); return true; }
            crate::editor::Action::SplineMode => {
                if self.net.lan.is_some() {
                    self.service_msg = Some(("Spline editing is available in single-player only".into(), 5.0));
                    return true;
                }
                self.menus.editor_drag = false;
                let ed = self.menus.editor.as_mut().unwrap();
                ed.placing_asset=None;ed.object_stamp=None;ed.terrain.active=false;
                ed.splines.cancel_connection();
                ed.splines.finish_drag();
                ed.spline_mode = !ed.spline_mode;
                ed.selected = None;
                ed.editing_added = None;
                if ed.spline_mode {
                    world.collect_editor_splines();
                    ed.describe(&world)
                } else { ed.describe(&world) }
            }
            crate::editor::Action::Generate => {
                let at = crate::editor::Editor::aim(&world, eye, fwd);
                let ed = self.menus.editor.as_mut().unwrap();
                if ed.spline_mode { ed.splines.generate(&world, at) }
                else { "Press T for spline mode".into() }
            }
            crate::editor::Action::Connect => {
                self.menus.editor_drag = false;
                let ed = self.menus.editor.as_mut().unwrap();
                if ed.spline_mode { ed.splines.connect_key(&world) }
                else { "T for splines, then click first spline and press G".into() }
            }
            crate::editor::Action::ReplaceConnection => {
                self.menus.editor_drag = false;
                let ed = self.menus.editor.as_mut().unwrap();
                if ed.spline_mode { ed.splines.replace_connection(&world) }
                else { "T for splines, then G to connect".into() }
            }
            a @ (crate::editor::Action::FitTerrain | crate::editor::Action::SmoothRoad(_) | crate::editor::Action::Branch(_) | crate::editor::Action::SnapEnd | crate::editor::Action::SplineUndo | crate::editor::Action::Split) => {
                let at = crate::editor::Editor::aim(&world, eye, fwd);
                let ed = self.menus.editor.as_mut().unwrap();
                if !ed.spline_mode { "This function requires spline mode (T)".into() }
                else {
                    ed.splines.finish_drag();
                    match a {
                        crate::editor::Action::FitTerrain => ed.splines.fit_terrain(&world),
                        crate::editor::Action::SmoothRoad(wide) => ed.splines.smooth_road(&world, wide),
                        crate::editor::Action::Branch(side) => ed.splines.branch(&world, at, side),
                        crate::editor::Action::SnapEnd if ed.splines.connection_active() => ed.splines.confirm_connection(&world),
                        crate::editor::Action::SnapEnd => ed.splines.snap_end(&world, at),
                        crate::editor::Action::Split => ed.splines.split(&world, at),
                        _ => ed.splines.undo(&world),
                    }
                }
            }
            crate::editor::Action::PlaceObject => { self.editor_open_catalog(Some(crate::asset_catalog::Kind::Object)); return true; }
            crate::editor::Action::Catalog => { self.editor_open_catalog(None); return true; }
            crate::editor::Action::Leave => {
                if self.menus.editor.as_mut().is_some_and(|ed| ed.splines.cancel_connection()) {
                    self.service_msg = Some(("Connection cancelled; nothing changed".into(), 4.0));
                    return true;
                }
                self.toggle_editor();
                return true;
            }
            crate::editor::Action::Pick => {
                let ed = self.menus.editor.as_mut().unwrap();
                if ed.spline_mode && ed.splines.connection_status().is_some_and(|(_, ready)| ready) {
                    ed.splines.confirm_connection(&world)
                } else {
                    ed.pick(&world, eye, fwd);
                    ed.describe(&world)
                }
            }
            crate::editor::Action::NextPick => {
                let ed = self.menus.editor.as_mut().unwrap();
                ed.next_pick();
                ed.splines.refresh_connection(&world);
                ed.describe(&world)
            }
            crate::editor::Action::Save => {
                let content = crate::startup::content_dir();
                let ed = self.menus.editor.as_ref().unwrap();
                let result = content.map(|c| ed.save(&world, &self.args.map, &c, &self.args.root));
                if result.as_ref().is_some_and(|r| r.is_ok()) { self.remember_editor_camera(); }
                match result {
                    Some(Ok(files)) if files.is_empty() => "Nothing to save".to_string(),
                    Some(Ok(files)) => {
                        let spline_changed = {
                            let e = world.spline_edits.lock();
                            !e.changed.is_empty() || !e.added.is_empty()
                        };
                        format!("{} file(s) saved in mod folder ({}){}", files.len(), files.iter().filter_map(|f| f.file_name()).map(|n| n.to_string_lossy()).collect::<Vec<_>>().join(", "),
                            if spline_changed { " - reload the map to update AI traffic" } else { "" })
                    }
                    Some(Err(e)) => format!("Not saved: {e}"),
                    None => "Not saved: no content folder".to_string(),
                }
            }
            a @ (crate::editor::Action::Ground(_) | crate::editor::Action::Flatten | crate::editor::Action::Brush(_)) => {
                let at = crate::editor::Editor::aim(&world, eye, fwd);
                let (msg, tiles) = self.menus.editor.as_mut().unwrap().ground(&world, at, &a);
                if !tiles.is_empty() {
                    log::info!("map editor: ground of tiles {tiles:?} at {at:?}");
                    self.editor_terrain_flush(0.0,true);
                }
                msg
            }
            a => {
                let (Some(r), Some(scene)) = (self.renderer.as_ref(), self.scene.as_mut()) else { return true };
                match self.menus.editor.as_mut().unwrap().apply(&world, r, scene, &a) {
                    Some(m) => m,
                    None => "Pick an object first (Enter)".to_string(),
                }
            }
        };
        log::info!("object editor: {msg}");
        self.service_msg = Some((msg, 5.0));
        self.editor_reload_splines();
        self.editor_broadcast(false);
        true
    }

    pub(crate) fn editor_open_terrain(&mut self) {
        if self.net.lan.is_some() || self.world.as_ref().is_some_and(|w|w.global.world_coordinates) {
            self.service_msg=Some(("Terrain mode requires single-player and a standard OMSI map".into(),5.0));return;
        }
        self.editor_terrain_finish();
        let (Some(ed),Some(w),Some(cam))=(self.menus.editor.as_mut(),self.world.as_ref(),self.camera.as_ref()) else {return;};
        ed.splines.finish_drag();ed.end_object_drag();ed.splines.cancel_connection();ed.splines.cancel_generation();
        ed.catalog=None;ed.texture_target=None;ed.junction_window=None;ed.roadside_window=None;ed.sidewalk_window=None;ed.tile_window=None;ed.text_window=None;ed.placing_asset=None;ed.object_stamp=None;
        ed.terrain.active=true;ed.terrain.input=None;
        ed.terrain.choose_layer(w,ed.terrain.texture_layer);
        if ed.terrain.tile.is_none() {
            let size=omsi_map::tile_size();let key=((cam.position.x/size).floor() as i32,(cam.position.y/size).floor() as i32);
            ed.terrain.tile=w.has_tile(key).then_some(key);
            ed.terrain.height=w.editor_terrain_height(cam.position.x,cam.position.y).unwrap_or(0.0);
        }
        self.menus.editor_drag=false;self.input.keys.clear();self.input.mouse_look=false;self.input.mmb_held=false;self.input.buttons_held=(false,false);
        if let Some(ui)=self.ui.as_mut() {ui.end_editor_dock_drag();}
        self.editor_reload_splines();
    }

    pub(crate) fn editor_terrain_finish(&mut self) {
        if let (Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref()) {ed.terrain.finish(world);}
        self.editor_terrain_flush(0.0,true);
    }

    pub(crate) fn editor_terrain_flush(&mut self,dt:f32,force:bool) {
        let Some(ed)=self.menus.editor.as_mut() else {return;};
        let mut tiles=ed.terrain.take_pending(dt,force);
        if force {
            if let Some(world) = self.world.as_ref() {
                if world.roadside_edits.lock().groups.iter().any(|g| g.settings.ground) {
                    if let Err(error) = crate::roadside_objects::refresh(world) { log::warn!("Object rows after terrain change: {error}"); }
                    tiles.extend(world.roadside_edits.lock().dirty_tiles.drain());
        tiles.extend(world.object_ground_dirty.lock().drain());
                    tiles.sort(); tiles.dedup();
                }
            }
        }
        if tiles.is_empty() {return;}
        if let (Some(st),Some(r),Some(scene))=(self.gfx.streamer.as_mut(),self.renderer.as_ref(),self.scene.as_mut()) {
            st.reload_editor(r,scene,&tiles,self.sound.audio.as_ref());
        } else if let (Some(w),Some(r),Some(scene))=(self.world.as_ref(),self.renderer.as_ref(),self.scene.as_mut()) {
            w.forget_staged(&tiles);
            for key in &tiles {w.unload_tile(r,scene,*key,self.sound.audio.as_ref());}
            let paths:Vec<_>=tiles.into_iter().filter_map(|(x,y)|w.tile_source(x,y).map(|p|(x,y,p))).collect();
            if let Err(e)=w.build_scene(r,scene,&paths) {log::warn!("terrain editor reload: {e}");}
        }
    }

    pub(crate) fn editor_terrain_command(&mut self,command:crate::terrain_editor::Command) {
        use crate::terrain_editor::Command;
        self.editor_terrain_finish();
        let (Some(ed),Some(w))=(self.menus.editor.as_mut(),self.world.as_ref()) else {return;};
        match command {
            Command::Exit=>{ed.terrain.active=false;ed.terrain.input=None;ed.terrain.cursor=None;ed.terrain.sample_height=false;ed.terrain.sample_texture=false;ed.terrain.pick_tile=false;},
            Command::Mode(mode)=>{ed.terrain.mode=mode;ed.terrain.input=None;ed.terrain.sample_height=false;ed.terrain.sample_texture=false;ed.terrain.pick_tile=false;},
            Command::TextureCatalog=>{self.editor_open_textures(crate::editor::TextureTarget::Terrain);return;},
            Command::Layer(delta)=>{let count=crate::ground_paint::layers(w).len();if count>0 {
                let layer=(ed.terrain.texture_layer as i64+delta as i64).rem_euclid(count as i64) as usize;ed.terrain.choose_layer(w,layer);}},
            Command::Erase(erase)=>{ed.terrain.texture_erase=erase;ed.terrain.sample_texture=false;},
            Command::SampleTexture=>{ed.terrain.sample_texture=true;ed.terrain.message="Click ground to sample its visible texture".into();},
            Command::Tool(tool)=>{ed.terrain.tool=tool;ed.terrain.sample_height=false;ed.terrain.pick_tile=false;},
            Command::FineSmooth=>{ed.terrain.tool=crate::terrain_editor::Tool::Smooth;ed.terrain.strength=0.15;ed.terrain.softness=80.0;ed.terrain.sample_height=false;ed.terrain.pick_tile=false;},
            Command::Edit(field)=>{ed.terrain.edit(field);self.input.keys.clear();},
            Command::Adjust(field,delta)=>{ed.terrain.input=None;let value=ed.terrain.value(field)+delta;ed.terrain.set_value(field,value);},
            Command::SampleHeight=>{ed.terrain.sample_height=true;ed.terrain.pick_tile=false;ed.terrain.message="Click terrain to sample its height".into();},
            Command::PickTile=>{ed.terrain.pick_tile=true;ed.terrain.sample_height=false;ed.terrain.message="Click the desired tile on the terrain".into();},
            Command::TileMove(sign)=>{ed.terrain.change_tile(w,ed.terrain.tile_step*sign,false);},
            Command::TileLevel=>{ed.terrain.change_tile(w,ed.terrain.height,true);},
            Command::Blend=>{ed.terrain.blend=!ed.terrain.blend;},
            Command::Undo=>ed.terrain.undo_redo(w,false), Command::Redo=>ed.terrain.undo_redo(w,true),
            Command::Save=>{self.editor_action(crate::editor::Action::Save);return;},
        }
        self.editor_terrain_flush(0.0,true);
    }

    /// Numeric text fields consume typing before any camera or editor shortcuts.
    pub(crate) fn editor_terrain_input(&mut self,code:Option<KeyCode>,text:Option<&str>,pressed:bool)->bool {
        if !self.menus.editor.as_ref().is_some_and(|ed|ed.terrain.active && ed.terrain.input.is_some()) {return false;}
        if let Some(code)=code {self.input.keys.remove(&code);}
        if !pressed {return true;}
        let terrain=&mut self.menus.editor.as_mut().unwrap().terrain;
        match code {
            Some(KeyCode::Escape)=>terrain.input=None,
            Some(KeyCode::Enter|KeyCode::NumpadEnter)=>{terrain.commit_input();},
            Some(KeyCode::Backspace)=>{let input=terrain.input.as_mut().unwrap();if input.replace {input.text.clear();} else {input.text.pop();}input.replace=false;},
            Some(KeyCode::Delete)=>{let input=terrain.input.as_mut().unwrap();input.text.clear();input.replace=false;},
            _=>{if let Some(text)=text {
                let typed:String=text.chars().filter(|c|c.is_ascii_digit()||matches!(*c,'-'|'+'|','|'.')).collect();
                if !typed.is_empty() {let input=terrain.input.as_mut().unwrap();if input.replace {input.text.clear();input.replace=false;}
                    if input.text.len()+typed.len()<=24 {input.text.push_str(&typed);}}
            }},
        }
        true
    }

    pub(crate) fn editor_terrain_frame(&mut self,dt:f32) {
        if !self.menus.editor.as_ref().is_some_and(|ed|ed.terrain.active) {self.editor_terrain_flush(dt,false);return;}
        if self.input.input_away || self.menus.game_menu.is_some() || self.vr_active()
            || self.menus.editor.as_ref().is_some_and(|ed|ed.catalog.is_some()||ed.tile_window.is_some()) {
            self.editor_terrain_finish();return;
        }
        let (Some(cam),Some(surface),Some(w))=(self.camera.as_ref(),self.gfx.surface.as_ref(),self.world.clone()) else {return;};
        let covered=self.ui.as_ref().is_some_and(|ui|ui.editor_panel_contains(self.input.cursor));
        let (o,d)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));
        let hit=if covered||self.input.mouse_look {None} else {crate::terrain_editor::TerrainEditor::ground_hit(&w,o,d.as_dvec3())};
        let terrain=&mut self.menus.editor.as_mut().unwrap().terrain;terrain.cursor=hit;
        if hit.is_none() {terrain.finish(&w);}
        if let Some(at)=hit {if terrain.painting() && terrain.input.is_none() {terrain.move_brush(&w,at,dt);}}
        self.editor_terrain_flush(dt,false);
    }

    pub(crate) fn editor_open_tiles(&mut self) {
        self.editor_terrain_finish();
        if self.net.lan.is_some() {
            self.service_msg = Some(("New tiles are currently available only in single-player".into(), 5.0)); return;
        }
        let (Some(ed), Some(world), Some(cam)) = (self.menus.editor.as_mut(), self.world.as_ref(), self.camera.as_ref()) else { return; };
        if world.global.world_coordinates {
            self.service_msg = Some(("New tiles are currently available only for standard OMSI maps".into(), 5.0)); return;
        }
        ed.splines.finish_drag(); ed.end_object_drag(); ed.splines.cancel_connection(); ed.splines.cancel_generation();
        ed.placing_asset = None;ed.object_stamp=None;ed.catalog = None;ed.text_window = None;ed.junction_window=None;ed.roadside_window=None;ed.sidewalk_window=None;ed.texture_target=None;
        ed.terrain.input=None;ed.terrain.cursor=None;ed.terrain.sample_height=false;ed.terrain.pick_tile=false;
        ed.tile_window = Some(crate::tile_editor::Window::new(world, cam.position, cam.yaw));
        self.menus.editor_drag = false; self.input.keys.clear(); self.input.mouse_look = false; self.input.mmb_held = false;
        self.input.buttons_held = (false, false);
        self.on_right(false);
        if let Some(ui) = self.ui.as_mut() { ui.end_editor_dock_drag(); }
        self.editor_reload_splines();
    }

    pub(crate) fn editor_tile_command(&mut self, command: crate::tile_editor::Command) {
        use crate::tile_editor::Command;
        let Some(window) = self.menus.editor.as_mut().and_then(|ed| ed.tile_window.as_mut()) else { return; };
        match command {
            Command::Close => { self.menus.editor.as_mut().unwrap().tile_window = None; }
            Command::Select(key) => window.select(key),
            Command::HeightMode(own)=>{window.own_height=own;window.height_edit=None;window.message=if own {"Set new tile height in the number field"} else {"Neighbouring edge heights will be used"}.into();},
            Command::HeightAdjust(delta)=>{window.height=(window.height+delta).clamp(crate::terrain_editor::HEIGHT_MIN,crate::terrain_editor::HEIGHT_MAX);window.height_edit=None;},
            Command::EditHeight=>{window.height_edit=Some(format!("{:.2}",window.height));window.height_replace=true;},
            Command::Pan((x,y)) => {
                window.center.0 = (window.center.0 + x).clamp(-1_000_000,1_000_000);
                window.center.1 = (window.center.1 + y).clamp(-1_000_000,1_000_000);
            }
            Command::Create => {
                if !window.can_create() { return; }
                let key = window.selected.unwrap();
                let Some(world) = self.world.clone() else { return; };
                let result = crate::startup::content_dir().ok_or_else(|| "No content folder configured".to_string())
                    .and_then(|content| crate::tile_editor::create(&world,key,&content,&self.args.map,&self.args.root,window.own_height.then_some(window.height)));
                match result {
                    Ok(path) => {
                        if let Some(st) = self.gfx.streamer.as_mut() {
                            st.add_editor_tile(key, path);
                            if let (Some(r),Some(scene)) = (self.renderer.as_ref(),self.scene.as_mut()) {
                                st.reload_editor(r,scene,&[key],self.sound.audio.as_ref());
                            }
                        } else if let (Some(r),Some(scene)) = (self.renderer.as_ref(),self.scene.as_mut()) {
                            if let Some(path) = world.tile_source(key.0,key.1) {
                                if let Err(e) = world.build_scene(r,scene,&[(key.0,key.1,path)]) {
                                    window.message = format!("Tile saved; rendering failed: {e}. Reload map.");
                                    window.known.insert(key); window.selected = None; return;
                                }
                            }
                        }
                        window.known.insert(key); window.selected = None; window.center = key;
                        window.message = format!("Tile ({}, {}) saved. Close and continue building the road.",key.0,key.1);
                    }
                    Err(e) => { log::warn!("tile editor: {e}"); window.message = format!("Not created: {e}"); }
                }
            }
        }
    }

    pub(crate) fn editor_tile_input(&mut self, code: Option<KeyCode>, text:Option<&str>, pressed: bool) -> bool {
        use crate::tile_editor::Command;
        if !self.menus.editor.as_ref().is_some_and(|ed| ed.tile_window.is_some()) { return false; }
        if let Some(code) = code { self.input.keys.remove(&code); }
        if !pressed { return true; }
        let window=self.menus.editor.as_mut().unwrap().tile_window.as_mut().unwrap();
        if window.height_edit.is_some() {
            match code {
                Some(KeyCode::Escape)=>window.height_edit=None,
                Some(KeyCode::Enter|KeyCode::NumpadEnter)=>{
                    match crate::terrain_editor::parse_number(window.height_edit.as_deref().unwrap(),crate::terrain_editor::HEIGHT_MIN,crate::terrain_editor::HEIGHT_MAX) {
                        Ok(value)=>{window.height=value;window.height_edit=None;},Err(e)=>window.message=e,
                    }
                },
                Some(KeyCode::Backspace)=>{let value=window.height_edit.as_mut().unwrap();if window.height_replace {value.clear();} else {value.pop();}window.height_replace=false;},
                Some(KeyCode::Delete)=>{window.height_edit.as_mut().unwrap().clear();window.height_replace=false;},
                _=>{if let Some(text)=text {
                    let typed:String=text.chars().filter(|c|c.is_ascii_digit()||matches!(*c,'-'|'+'|','|'.')).collect();
                    if !typed.is_empty() {let value=window.height_edit.as_mut().unwrap();if window.height_replace {value.clear();window.height_replace=false;}
                        if value.len()+typed.len()<=24 {value.push_str(&typed);}}
                }},
            }
            return true;
        }
        let command = match code {
            Some(KeyCode::Escape) => Some(Command::Close),
            Some(KeyCode::Enter | KeyCode::NumpadEnter) => Some(Command::Create),
            Some(KeyCode::ArrowLeft) => Some(Command::Pan((-1,0))),
            Some(KeyCode::ArrowRight) => Some(Command::Pan((1,0))),
            Some(KeyCode::ArrowUp) => Some(Command::Pan((0,1))),
            Some(KeyCode::ArrowDown) => Some(Command::Pan((0,-1))),
            _ => None,
        };
        if let Some(command) = command { self.editor_tile_command(command); }
        true
    }

    pub(crate) fn editor_text_command(&mut self, command: crate::object_text::Command) {
        use crate::object_text::Command;
        let Some(mut window) = self.menus.editor.as_mut().and_then(|ed| ed.text_window.take()) else { return; };
        match command {
            Command::Close => { self.input.keys.clear(); return; }
            Command::Field(index) => window.select(index),
            Command::Page(delta) => {
                let page = (window.page() as i64 + delta as i64).clamp(0, window.fields.len().div_ceil(6) as i64 - 1);
                window.select(page as usize * 6);
            }
            Command::Apply | Command::Undo => {
                if let (Some(ed), Some(world), Some(r), Some(scene)) =
                    (self.menus.editor.as_mut(), self.world.as_ref(), self.renderer.as_ref(), self.scene.as_mut()) {
                    let result = if matches!(command, Command::Apply) {
                        ed.set_labels(world, r, scene, window.target, window.values.clone(), true)
                    } else {
                        ed.undo_labels(world, r, scene, window.target).map(|mut values| {
                            let count = window.fields.iter().map(|(slot,_)|slot+1).max().unwrap_or(0);
                            values.resize(values.len().max(count), String::new()); window.values = values;
                        })
                    };
                    window.message = match result { Ok(()) => "Text applied · Ctrl+S saves the map".into(), Err(e) => e };
                }
            }
        }
        self.menus.editor.as_mut().unwrap().text_window = Some(window);
    }

    pub(crate) fn editor_text_input(&mut self, code: Option<KeyCode>, text: Option<&str>, pressed: bool) -> bool {
        use crate::object_text::Command;
        if !self.menus.editor.as_ref().is_some_and(|ed| ed.text_window.is_some()) { return false; }
        if let Some(code) = code {
            if matches!(code,KeyCode::ControlLeft|KeyCode::ControlRight|KeyCode::ShiftLeft|KeyCode::ShiftRight) && pressed {
                self.input.keys.insert(code); return true;
            }
            self.input.keys.remove(&code);
        }
        if !pressed { return true; }
        let ctrl = self.input.keys.contains(&KeyCode::ControlLeft) || self.input.keys.contains(&KeyCode::ControlRight);
        let shift = self.input.keys.contains(&KeyCode::ShiftLeft) || self.input.keys.contains(&KeyCode::ShiftRight);
        let command = match code {
            Some(KeyCode::Escape) => Some(Command::Close),
            Some(KeyCode::Enter|KeyCode::NumpadEnter) => Some(Command::Apply),
            Some(KeyCode::KeyZ) if ctrl => Some(Command::Undo),
            _ => None,
        };
        if let Some(command) = command { self.editor_text_command(command); return true; }
        if code == Some(KeyCode::KeyS) && ctrl { self.editor_text_command(Command::Apply); self.editor_action(crate::editor::Action::Save); return true; }
        let window = self.menus.editor.as_mut().unwrap().text_window.as_mut().unwrap();
        match code {
            Some(KeyCode::Tab) => window.next(if shift { -1 } else { 1 }),
            Some(KeyCode::Backspace) => window.erase(false),
            Some(KeyCode::Delete) => window.erase(true),
            Some(KeyCode::KeyA) if ctrl => window.replace = true,
            Some(KeyCode::KeyV) if ctrl => {
                #[cfg(not(target_os = "android"))]
                if let Ok(mut clipboard) = arboard::Clipboard::new() { if let Ok(text) = clipboard.get_text() { window.type_text(&text); } }
            }
            _ if !ctrl => { if let Some(text) = text { window.type_text(text); } }
            _ => {}
        }
        true
    }

    pub(crate) fn editor_open_catalog(&mut self, kind: Option<crate::asset_catalog::Kind>) {
        self.editor_terrain_finish();
        let (Some(ed), Some(world)) = (self.menus.editor.as_mut(), self.world.as_ref()) else { return; };
        ed.splines.cancel_connection(); ed.splines.finish_drag(); ed.splines.cancel_generation();
        ed.placing_asset = None;ed.object_stamp=None;
        ed.tile_window = None;
        ed.text_window = None;
        ed.junction_window = None;ed.roadside_window=None;ed.sidewalk_window=None;ed.texture_target=None;
        ed.terrain.active = false;
        let kind = kind.unwrap_or(if ed.spline_mode { crate::asset_catalog::Kind::Spline } else { crate::asset_catalog::Kind::Object });
        ed.catalog = Some(crate::asset_catalog::Catalog::with_map(world.root.clone(), kind,Some(world.map_dir.clone())));
        self.menus.editor_drag = false; self.input.dragging = false; self.input.mouse_look = false;
        self.input.mmb_held = false; self.input.cursor_hidden = None;
        if let Some(window) = self.window.as_ref() { window.set_cursor_visible(true); }
        self.input.buttons_held = (false, false); self.input.both_drag = None;
        self.release_vehicle_keys(); self.input.keys.clear();
        self.service_msg = Some(("Asset catalogue: type search · Click tile · Select · Esc closes".into(), 8.0));
    }

    pub(crate) fn editor_catalog_command(&mut self, command: crate::asset_catalog::Command) {
        use crate::asset_catalog::{Command, Kind};
        let Some(ed) = self.menus.editor.as_mut() else { return; };
        let Some(catalog) = ed.catalog.as_mut() else { return; };
        match command {
            Command::Close => { ed.catalog = None;ed.texture_target=None; }
            Command::BuildJunction=>{self.editor_open_junction();return;}
            Command::Choose => {
                let Some(asset) = catalog.chosen() else { return; };
                if let Some(window)=ed.sidewalk_window.as_mut(){
                    if asset.kind!=Kind::Spline{self.service_msg=Some(("Select a sidewalk spline profile".into(),5.0));return;}
                    window.file=asset.file;window.existing=None;window.message="Profile and textures selected – check preview".into();
                    if let Some(world)=self.world.as_ref(){window.refresh(world);}ed.catalog=None;self.input.keys.clear();self.input.buttons_held=(false,false);return;
                }
                if let Some(window) = ed.roadside_window.as_mut() {
                    if asset.kind != Kind::Object { self.service_msg = Some(("Choose a scenery object for an object row".into(), 5.0)); return; }
                    window.file = asset.file; window.message = "Object selected · Blue = planned positions".into();
                    if let Some(world) = self.world.as_ref() { window.refresh(world); }
                    ed.catalog = None; self.input.keys.clear(); self.input.buttons_held = (false, false); return;
                }
                if asset.kind==Kind::Texture {
                    let target=ed.texture_target.unwrap_or(crate::editor::TextureTarget::Terrain);
                    if self.net.lan.is_some() {self.service_msg=Some(("Texture editing is available in single-player".into(),5.0));return;}
                    let Some(world)=self.world.as_ref() else {return;};
                    match target {
                        crate::editor::TextureTarget::Terrain=>{
                            if world.global.world_coordinates {self.service_msg=Some(("Texture brush requires a standard OMSI map".into(),5.0));return;}
                            match crate::ground_paint::select_texture(world,&asset,4.0) {
                                Ok(layer)=>{ed.terrain.active=true;ed.terrain.mode=crate::terrain_editor::Mode::Textures;
                                    ed.terrain.input=None;ed.terrain.sample_height=false;ed.terrain.pick_tile=false;ed.terrain.cursor=None;
                                    ed.placing_asset=None;ed.object_stamp=None;ed.junction_window=None;
                                    ed.terrain.choose_layer(world,layer);ed.terrain.texture_erase=false;
                                    log::info!("Texture brush: selected layer {layer} '{}'",asset.file);
                                    ed.terrain.message="Texture selected · Hold left mouse button to paint · Ctrl+Z to undo · Ctrl+S to save".into();},
                                Err(error)=>{log::warn!("Texture brush: '{}' not selected: {error}",asset.file);self.service_msg=Some((error,8.0));return;}
                            }
                        }
                        crate::editor::TextureTarget::JunctionRoad|crate::editor::TextureTarget::JunctionWalk=>{
                            if let Some(window)=ed.junction_window.as_mut() {window.texture(asset.file,matches!(target,crate::editor::TextureTarget::JunctionWalk));}
                        }
                    }
                    ed.catalog=None;ed.texture_target=None;self.input.keys.clear();self.input.buttons_held=(false,false);
                    self.input.mouse_look=false;self.input.mmb_held=false;self.menus.editor_drag=false;return;
                }
                if asset.kind == Kind::Spline && self.net.lan.is_some() {
                    self.service_msg = Some(("Spline editing is available in single-player only".into(), 5.0)); return;
                }
                if asset.kind == Kind::Spline { ed.splines.choose_file(asset.file.clone()); ed.spline_mode = true; ed.selected = None; ed.editing_added = None; }
                else {ed.spline_mode=false;ed.object_stamp=Some(crate::editor::ObjectStamp::new(asset.path.clone()));
                    ed.align_object=asset.category==crate::asset_catalog::Category::Junctions;}
                let message = if asset.kind == Kind::Spline { "Road type selected · Click start on ground, then target · Esc/B cancels" }
                    else { "Object selected · Click target · Repeat placement for more copies · Esc to finish" };
                ed.placing_asset = Some(asset); ed.catalog = None;
                self.service_msg = Some((message.into(), 12.0));
            }
            command => catalog.command(command),
        }
    }

    /// A modal catalogue owns all keyboard input, including layout-produced text.
    pub(crate) fn editor_catalog_input(&mut self, code: Option<KeyCode>, text: Option<&str>, pressed: bool) -> bool {
        use crate::asset_catalog::{Command, Section};
        if !self.menus.editor.as_ref().is_some_and(|ed| ed.catalog.is_some()) { return false; }
        if !pressed { if let Some(code) = code { self.input.keys.remove(&code); } return true; }
        if self.menus.editor.as_ref().unwrap().catalog.as_ref().unwrap().info_open {
            let command=match code {
                Some(KeyCode::Escape)=>Some(Command::TextureInfo),
                Some(KeyCode::ArrowLeft|KeyCode::PageUp)=>Some(Command::InfoStep(-1)),
                Some(KeyCode::ArrowRight|KeyCode::PageDown)=>Some(Command::InfoStep(1)),_=>None};
            if let Some(command)=command {self.editor_catalog_command(command);}return true;
        }
        let command = match code {
            Some(KeyCode::Escape) => Some(Command::Close),
            Some(KeyCode::Enter | KeyCode::NumpadEnter) => Some(Command::Choose),
            Some(KeyCode::PageUp) => Some(Command::Page(-1)), Some(KeyCode::PageDown) => Some(Command::Page(1)),
            Some(KeyCode::Tab) => self.menus.editor.as_ref().and_then(|ed| ed.catalog.as_ref()).map(|c| Command::Section(if c.section == Section::Roads { Section::Objects } else { Section::Roads })),
            _ => None,
        };
        if let Some(command) = command { self.editor_catalog_command(command); return true; }
        let catalog = self.menus.editor.as_mut().unwrap().catalog.as_mut().unwrap();
        match code {
            Some(KeyCode::Backspace) => catalog.backspace(),
            Some(KeyCode::ArrowLeft) => catalog.move_selection(-1), Some(KeyCode::ArrowRight) => catalog.move_selection(1),
            Some(KeyCode::ArrowUp) => catalog.move_selection(-4), Some(KeyCode::ArrowDown) => catalog.move_selection(4),
            _ => { if let Some(text) = text { catalog.type_text(text); } }
        }
        true
    }

    /// Rebuild the surfaces when an edit is committed. During a drag only the markers
    /// follow the cursor; release rebuilds once, rather than reloading at every pixel.
    pub(crate) fn editor_reload_splines(&mut self) {
        let Some(world) = self.world.clone() else { return; };
        let (changed, mut tiles): (bool, hashbrown::HashSet<_>) = {
            let mut edits = world.spline_edits.lock();
            let changed = edits.dirty; edits.dirty = false;
            (changed, edits.dirty_tiles.drain().collect())
        };
        if changed {
            if let Err(error) = crate::roadside_objects::initialize(&world).and_then(|_| crate::roadside_objects::refresh(&world)) {
                log::warn!("Object rows after road change: {error}");
                self.service_msg = Some((format!("Check object rows: {error}"), 6.0));
            }
        }
        tiles.extend(world.roadside_edits.lock().dirty_tiles.drain());
        tiles.extend(world.object_ground_dirty.lock().drain());
        if tiles.is_empty() { return; }
        let mut tiles: Vec<_> = tiles.into_iter().collect(); tiles.sort();
        if let (Some(st), Some(r), Some(scene)) = (self.gfx.streamer.as_mut(), self.renderer.as_ref(), self.scene.as_mut()) {
            st.reload_editor(r, scene, &tiles, self.sound.audio.as_ref());
        }
    }

    /// The host's edits to the other players' games (`all`: every edit of the session,
    /// sent again every ten seconds for the ones who joined since).
    pub(crate) fn editor_broadcast(&mut self, all: bool) {
        let (Some(ed), Some(w)) = (self.menus.editor.as_ref(), self.world.as_ref()) else {
            if all {
                // (edits stay after the editor is left: sent from the world's list)
                if let (Some(w), Some(l)) = (self.world.as_ref(), self.net.lan.as_mut()) {
                    if l.role == omsi_net::Role::Host {
                        let lines = self.menus.editor_paused.as_ref().map(|ed| ed.sync_lines(w, &self.args.root, true))
                            .unwrap_or_else(|| crate::editor::Editor::default().sync_lines(w, &self.args.root, true));
                        let ids: Vec<u32> = l.peers().map(|p| p.pose.id).filter(|id| *id != l.my_id).collect();
                        for line in &lines {
                            for id in &ids {
                                l.command(*id, line);
                            }
                        }
                    }
                }
            }
            return;
        };
        let Some(l) = self.net.lan.as_mut() else { return };
        if l.role != omsi_net::Role::Host {
            return;
        }
        let lines = ed.sync_lines(w, &self.args.root, all);
        let ids: Vec<u32> = l.peers().map(|p| p.pose.id).filter(|id| *id != l.my_id).collect();
        for line in &lines {
            if line.len() > omsi_net::MAX_CHAT {
                log::warn!("object editor: '{line}' is too long to send");
                continue;
            }
            for id in &ids {
                l.command(*id, line);
            }
        }
    }

    /// The mouse in the object editor: a click picks what is under the cursor (and starts
    /// dragging it), a drag moves it over the ground; true when the editor took it.
    pub(crate) fn editor_mouse(&mut self, pressed: bool) -> bool {
        if self.menus.editor.is_none() {
            return false;
        }
        if let Some(window) = self.menus.editor.as_ref().and_then(|ed| ed.text_window.as_ref()) {
            let command = if pressed { window.hit(self.input.cursor) } else { None };
            if let Some(command) = command { self.editor_text_command(command); }
            return true;
        }
        if !pressed {self.editor_terrain_finish();}
        if !pressed && self.ui.as_mut().is_some_and(|ui| ui.end_editor_dock_drag()) {
            return true;
        }
        if let Some(window) = self.menus.editor.as_ref().and_then(|ed| ed.tile_window.as_ref()) {
            let command = if pressed { window.hit(self.input.cursor) } else { None };
            if let Some(command) = command { self.editor_tile_command(command); }
            return true;
        }
        if let Some(catalog) = self.menus.editor.as_ref().and_then(|ed| ed.catalog.as_ref()) {
            let command = pressed.then(|| catalog.hit(self.input.cursor)).flatten();
            if let Some(command) = command { self.editor_catalog_command(command); }
            return true;
        }
        if let Some(window)=self.menus.editor.as_ref().and_then(|ed|ed.junction_window.as_ref()) {
            let command=if pressed {window.hit(self.input.cursor)} else {None};
            if let Some(command)=command {self.editor_junction_command(command);}return true;
        }
        if self.editor_sidewalk_mouse(pressed){return true;}
        if self.editor_roadside_mouse(pressed) { return true; }
        if self.menus.editor.as_ref().is_some_and(crate::audit_events::panel_active) {
            let a=self.menus.editor.as_ref().and_then(|ed|ed.audit.as_ref()).filter(|a|a.visible);
            if a.and_then(|a|a.rect).is_some_and(|r|crate::audit_events::contains(r,self.input.cursor)) {
                let command=if pressed {a.and_then(|a|a.hit(self.input.cursor))}else{None};
                if let Some(command)=command{self.editor_audit_command(command);}return true;
            }
        }
        if pressed && self.ui.as_ref().and_then(|ui|ui.editor_audit_rect).is_some_and(|r|crate::audit_events::contains(r,self.input.cursor)) {
            self.editor_action(crate::editor::Action::AuditSplines);return true;
        }
        if pressed && self.ui.as_ref().and_then(|ui|ui.editor_reload_rect).is_some_and(|r|
            self.input.cursor.0>=r[0]&&self.input.cursor.0<=r[2]&&self.input.cursor.1>=r[1]&&self.input.cursor.1<=r[3]) {
            self.editor_action(crate::editor::Action::ReloadMap);return true;
        }
        if pressed && self.ui.as_mut().is_some_and(|ui| ui.editor_hud_press(self.input.cursor)) {
            return true;
        }
        if pressed && self.ui.as_ref().and_then(|ui|ui.terrain_open_rect).is_some_and(|r|
            self.input.cursor.0>=r[0] && self.input.cursor.0<=r[2] && self.input.cursor.1>=r[1] && self.input.cursor.1<=r[3]) {
            self.editor_action(crate::editor::Action::TerrainMode);return true;
        }
        if pressed && self.ui.as_ref().and_then(|ui|ui.roundabout_open_rect).is_some_and(|r|
            self.input.cursor.0>=r[0]&&self.input.cursor.0<=r[2]&&self.input.cursor.1>=r[1]&&self.input.cursor.1<=r[3]) {
            self.editor_action(crate::editor::Action::RoundaboutWindow);return true;
        }
        if pressed && self.ui.as_ref().and_then(|ui|ui.junction_open_rect).is_some_and(|r|
            self.input.cursor.0>=r[0] && self.input.cursor.0<=r[2] && self.input.cursor.1>=r[1] && self.input.cursor.1<=r[3]) {
            self.editor_action(crate::editor::Action::JunctionWindow);return true;
        }
        if pressed {
            let command=self.ui.as_ref().and_then(|ui|ui.terrain_tool_rects.iter().find(|(r,_)|
                self.input.cursor.0>=r[0] && self.input.cursor.0<=r[2] && self.input.cursor.1>=r[1] && self.input.cursor.1<=r[3]).map(|(_,c)|*c));
            if let Some(command)=command {self.editor_terrain_command(command);return true;}
        }
        if pressed && self.ui.as_ref().and_then(|ui| ui.tile_open_rect).is_some_and(|r|
            self.input.cursor.0 >= r[0] && self.input.cursor.0 <= r[2] && self.input.cursor.1 >= r[1] && self.input.cursor.1 <= r[3]) {
            self.editor_action(crate::editor::Action::TileWindow); return true;
        }
        if pressed && self.ui.as_ref().and_then(|ui| ui.catalog_open_rect).is_some_and(|r|
            self.input.cursor.0 >= r[0] && self.input.cursor.0 <= r[2] && self.input.cursor.1 >= r[1] && self.input.cursor.1 <= r[3]) {
            self.editor_open_catalog(None); return true;
        }
        if pressed {
            let action = self.ui.as_ref().and_then(|ui| ui.spline_tool_rects.iter().find(|(r, _)|
                self.input.cursor.0 >= r[0] && self.input.cursor.0 <= r[2] && self.input.cursor.1 >= r[1] && self.input.cursor.1 <= r[3])
                .map(|(_, action)| *action));
            if let Some(action) = action { self.editor_action(action); return true; }
        }
        if !pressed {
            if self.menus.editor_drag {
                self.menus.editor_drag = false;
                if let Some(ed) = self.menus.editor.as_mut() { ed.splines.finish_drag(); ed.end_object_drag(); }
                self.editor_reload_splines();
                self.editor_broadcast(false);
            }
            return true;
        }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.spline_mode && ed.splines.connection_active()) {
            let inside = |rect: [f32; 4]| self.input.cursor.0 >= rect[0] && self.input.cursor.0 <= rect[2]
                && self.input.cursor.1 >= rect[1] && self.input.cursor.1 <= rect[3];
            if self.ui.as_ref().is_some_and(|ui| ui.spline_transition_rect.is_some_and(inside)) {
                if let (Some(ed), Some(world)) = (self.menus.editor.as_mut(), self.world.as_ref()) {
                    self.service_msg = Some((ed.splines.toggle_transition(world), 5.0));
                }
                return true;
            }
            let hit = self.ui.as_ref().and_then(|ui| {
                if ui.spline_connect_rect.is_some_and(inside) { Some(true) }
                else if ui.spline_cancel_rect.is_some_and(inside) { Some(false) } else { None }
            });
            if let (Some(confirm), Some(world)) = (hit, self.world.clone()) {
                self.menus.editor_drag = false;
                let ed = self.menus.editor.as_mut().unwrap();
                let message = if confirm && ed.splines.connection_can_replace() { ed.splines.replace_connection(&world) }
                    else if confirm { ed.splines.confirm_connection(&world) }
                    else { ed.splines.cancel_connection(); "Connection cancelled; nothing changed".into() };
                self.service_msg = Some((message, 5.0));
                self.editor_reload_splines();
                return true;
            }
        }
        if self.ui.as_ref().is_some_and(|ui| ui.editor_panel_contains(self.input.cursor)) {
            return true;
        }
        let (Some(cam), Some(s), Some(world)) = (self.camera.as_ref(), self.gfx.surface.as_ref(), self.world.clone()) else { return true };
        let (o, d) = self.world_cursor_ray(cam, (s.config.width, s.config.height));
        if self.menus.editor.as_ref().is_some_and(|ed|ed.terrain.active) {
            let terrain=&mut self.menus.editor.as_mut().unwrap().terrain;
            if !terrain.commit_input() {return true;}
            if let Some(hit)=crate::terrain_editor::TerrainEditor::ground_hit(&world,o,d.as_dvec3()) {terrain.begin(&world,hit);}
            else {terrain.message="Click loaded terrain inside the map".into();
                if terrain.mode==crate::terrain_editor::Mode::Textures {log::info!("Texture brush: no terrain hit · {}",terrain.message);}}
            return true;
        }
        if let Some(asset) = self.menus.editor.as_ref().and_then(|ed| ed.placing_asset.clone()) {
            self.menus.editor_drag = false;
            let Some(hit) = crate::placing::ground_hit(&world, o, d.as_dvec3(), 400.0) else {
                self.service_msg = Some(("Click a ground location inside the map".into(), 4.0)); return true;
            };
            let object=asset.kind==crate::asset_catalog::Kind::Object;
            let message = if asset.kind == crate::asset_catalog::Kind::Spline {
                let ed = self.menus.editor.as_mut().unwrap(); let started = ed.splines.generation_started();
                let message = ed.splines.generate(&world, Some(hit));
                if started && !ed.splines.generation_started() { ed.placing_asset = None; }
                if !started { "Start selected (blue marker) · Now click target · Esc/B cancels".into() } else { message }
            } else {
                let (Some(r), Some(scene)) = (self.renderer.as_ref(), self.scene.as_mut()) else { return true; };
                let ed = self.menus.editor.as_mut().unwrap();
                let mut stamp=ed.object_stamp.clone().unwrap_or_else(||crate::editor::ObjectStamp::new(asset.path));
                let (mut at,heading)=if ed.align_object {ed.splines.junction_pose(&world,hit)} else {(hit,stamp.heading)};
                at.z+=stamp.height_offset;stamp.heading=heading;
                let message=if stamp.tilt==[0.0;2] && stamp.strings.is_empty() {ed.place_object(&world,r,scene,stamp.sco.clone(),at,heading)}
                    else {ed.place_object_values(&world,r,scene,stamp.sco.clone(),at,heading,stamp.tilt,stamp.strings.clone())};
                if message.starts_with("Object placed") {
                    ed.last_object=Some(stamp.clone());ed.clipboard=Some(stamp);
                    if !ed.repeat_objects {ed.placing_asset=None;ed.object_stamp=None;}
                    else {ed.object_stamp=ed.last_object.clone();}
                }
                message
            };
            self.service_msg = if object && message.starts_with("Object placed") {None} else {Some((message,8.0))};
            self.editor_reload_splines(); self.editor_broadcast(false);
            return true;
        }
        let ed = self.menus.editor.as_mut().unwrap();
        if ed.spline_mode {
            ed.pick(&world, o, d);
            if !ed.splines.connection_active() {
                if let Some(hit) = crate::placing::ground_hit(&world, o, d.as_dvec3(), 400.0) {
                ed.splines.begin_drag(&world, hit);
                }
            }
            self.menus.editor_drag = !ed.splines.connection_active() && ed.splines.selected.is_some();
            self.service_msg = Some((ed.describe(&world), 5.0));
            return true;
        }
        ed.pick(&world, o, d);
        self.menus.editor_drag = crate::placing::ground_hit(&world, o, d.as_dvec3(), 400.0)
            .is_some_and(|ground| ed.begin_object_drag(&world, ground));
        let msg = ed.describe(&world);
        self.service_msg = Some((msg, 5.0));
        true
    }

    /// The cursor moved while an object is dragged.
    pub(crate) fn editor_drag_frame(&mut self) {
        if !self.menus.editor_drag {
            return;
        }
        let (Some(cam), Some(s), Some(world)) = (self.camera.as_ref(), self.gfx.surface.as_ref(), self.world.clone()) else { return };
        let (o, d) = self.world_cursor_ray(cam, (s.config.width, s.config.height));
        let Some(hit) = crate::placing::ground_hit(&world, o, d.as_dvec3(), 400.0) else { return };
        let (Some(r), Some(scene), Some(ed)) = (self.renderer.as_ref(), self.scene.as_mut(), self.menus.editor.as_mut()) else { return };
        if let Some(m) = ed.drag_to(&world, r, scene, hit) {
            self.service_msg = Some((m, 3.0));
        }
    }

    /// The wheel in the object editor: the object turns (5° a notch), with Shift it rises.
    pub(crate) fn editor_wheel(&mut self, amount: f32) -> bool {
        if self.menus.editor.is_none() {
            return false;
        }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.text_window.is_some()) {
            if amount != 0.0 { self.editor_text_command(crate::object_text::Command::Page(if amount > 0.0 { -1 } else { 1 })); }
            return true;
        }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.tile_window.is_some()) { return true; }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.catalog.is_some()) {
            if amount != 0.0 { let info=self.menus.editor.as_ref().and_then(|e|e.catalog.as_ref()).is_some_and(|c|c.info_open);
                self.editor_catalog_command(if info {crate::asset_catalog::Command::InfoStep(if amount>0.0 {-1} else {1})} else {crate::asset_catalog::Command::Page(if amount > 0.0 { -1 } else { 1 })}); }
            return true;
        }
        if self.menus.editor.as_ref().is_some_and(|ed|ed.sidewalk_window.is_some() || ed.junction_window.is_some() || ed.roadside_window.is_some()) {return true;}
        if self.ui.as_mut().is_some_and(|ui| ui.editor_dock_scroll(self.input.cursor, amount)) {
            return true;
        }
        if self.ui.as_ref().is_some_and(|ui|ui.editor_panel_contains(self.input.cursor)) {return true;}
        if self.menus.editor.as_ref().is_some_and(|ed|ed.terrain.active) {
            if amount.is_finite() {self.editor_terrain_finish();let terrain=&mut self.menus.editor.as_mut().unwrap().terrain;
                terrain.set_value(crate::terrain_editor::Field::Radius,terrain.radius*1.12_f64.powf(amount as f64));}
            return true;
        }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.placing_asset.is_some()) {
            let shift=self.input.keys.contains(&KeyCode::ShiftLeft)||self.input.keys.contains(&KeyCode::ShiftRight);
            let ed=self.menus.editor.as_mut().unwrap();if let Some(stamp)=ed.object_stamp.as_mut() {
                if amount.is_finite() {if shift {stamp.height_offset+=0.1*amount as f64;} else {stamp.heading+=5.0*amount as f64;}ed.align_object=false;}
            }return true;
        }
        if self.menus.editor.as_ref().is_some_and(|ed| ed.spline_mode && ed.splines.connection_active()) {
            return true;
        }
        let shift = self.input.keys.contains(&KeyCode::ShiftLeft) || self.input.keys.contains(&KeyCode::ShiftRight);
        let action = if shift { crate::editor::Action::Move(glam::DVec3::Z * 0.1 * amount as f64) } else { crate::editor::Action::Turn(5.0 * amount as f64) };
        let (Some(world), Some(r), Some(scene)) = (self.world.clone(), self.renderer.as_ref(), self.scene.as_mut()) else { return true };
        if let Some(m) = self.menus.editor.as_mut().unwrap().apply(&world, r, scene, &action) {
            self.service_msg = Some((m, 3.0));
            self.editor_reload_splines();
            self.editor_broadcast(false);
        }
        true
    }
}
