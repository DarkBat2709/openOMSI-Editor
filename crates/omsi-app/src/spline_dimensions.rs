//! Numeric spline dimensions. Presets apply to newly placed splines; existing roads
//! change only after Apply, through the editor's normal undo/save path.
use crate::{app::App, editor::Editor};
use winit::keyboard::KeyCode;
#[derive(Clone, Copy)]
pub enum Command { Edit(usize), Read, Apply, Preset, Close }
pub struct Window {
    pub values: [f64;4],
    pub heading:f64, pub anchor:Option<glam::DVec3>, pub angle_mode:bool,
    pub input: Option<(usize,String,bool)>,
    pub rect: Option<[f32;4]>,
    pub buttons: Vec<([f32;4],Command)>,
    pub message: String,
}
impl Default for Window {
    fn default()->Self { Self { values:[5.0,0.0,0.0,0.0],heading:0.0,anchor:None,angle_mode:false,input:None,rect:None,buttons:Vec::new(),message:String::new() } }
}
pub fn visible(ed:&Editor)->bool {
    ed.spline_mode && ed.catalog.is_none() && ed.junction_window.is_none() && ed.construction_window.is_none()
        && ed.sidewalk_window.is_none() && ed.roadside_window.is_none() && ed.traffic_window.is_none()
        && ed.tile_window.is_none() && ed.text_window.is_none() && !ed.terrain.active
}
pub use omsi_geometry::spline_dimensions::{validate, reshape};
impl Window {
    pub fn value(&self,i:usize)->f64 {if i==4 {omsi_geometry::spline_dimensions::turn_degrees(self.values)}else{self.values[i]}}
    pub fn live(&self)->Result<[f64;4],String> {
        let Some((i,text,_))=&self.input else{return Ok(self.values);};
        let n=text.replace(',',".").parse::<f64>().map_err(|_|"Enter a valid number")?;
        let mut v=omsi_geometry::spline_dimensions::edit_dimension(self.values,*i,n)?;
        if *i==0 && self.angle_mode {v=omsi_geometry::spline_dimensions::edit_dimension(v,4,self.value(4))?;} Ok(v)
    }
    fn commit(&mut self)->bool {
        let v=match self.live(){Ok(v)=>v,Err(e)=>{self.message=e;return false;}};
        if let Some((i,_,_))=self.input {if i==4 {self.angle_mode=true;}else if i==1 {self.angle_mode=false;}}
        self.values=v;self.input=None;true
    }
}
impl App {
    pub(crate) fn editor_open_dimensions(&mut self) {
        if self.net.lan.is_some(){return;}
        let (Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref()) else{return;};
        ed.splines.finish_drag();
        let mut w=Window::default();
        if let Some(v)=ed.splines.dimensions {w.values=v;} else if let Some(s)=ed.splines.selected.and_then(|k|world.spline_edits.lock().current(k)) {
            w.values=[s.length,s.radius,s.grad_start,s.pos[2]];w.heading=s.heading;
        }
        w.message="Radius 0 = straight · Height = absolute start height".into();
        ed.dimensions_window=Some(w);
    }
    fn editor_dimensions_command(&mut self,c:Command) {
        let (Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref()) else{return;};
        let Some(w)=ed.dimensions_window.as_mut() else{return;};
        if matches!(c,Command::Close){close(ed);self.input.keys.clear();return;}
        if !w.commit(){return;}
        if ed.splines.dimensions.is_some(){ed.splines.dimensions=Some(w.values);}
        match c {
            Command::Edit(i)=>{w.input=Some((i,format!("{:.4}",w.value(i)),true));self.input.keys.clear();}
            Command::Read=>{
                if let Some(s)=ed.splines.selected.and_then(|k|world.spline_edits.lock().current(k)) {
                    w.values=[s.length,s.radius,s.grad_start,s.pos[2]];w.heading=s.heading;w.angle_mode=false;w.message="Selected spline values loaded".into();
                } else {w.message="Select a road first".into();}
            }
            Command::Apply=>{
                w.message=match ed.splines.apply_dimensions(world,w.values){Ok(())=>"Spline dimensions applied · Ctrl+Z undo · Ctrl+S save".into(),Err(e)=>e};
            }
            Command::Preset=>{
                if ed.splines.dimensions.is_some(){ed.splines.dimensions=None;}
                else if let Err(e)=validate(w.values){w.message=e;return;}else{ed.splines.dimensions=Some(w.values);}
                w.message="Preview follows mouse · N/M turn · Click places · Esc finishes".into();
            }
            Command::Close=>{}
        }
        if ed.splines.dimensions.is_some(){ed.splines.dimensions=Some(w.values);}
    }
    pub(crate) fn editor_dimensions_mouse(&mut self,pressed:bool)->bool {
        let Some(ed)=self.menus.editor.as_mut().filter(|ed|visible(ed)) else{return false;}; let Some(w)=ed.dimensions_window.as_mut() else{return false;};
        if !w.rect.is_some_and(|r|crate::audit_events::contains(r,self.input.cursor)){if pressed {if !w.commit(){return true;}if ed.splines.dimensions.is_some(){ed.splines.dimensions=Some(w.values);}}return false;}
        let cmd=pressed.then(||w.buttons.iter().find(|(r,_)|crate::audit_events::contains(*r,self.input.cursor)).map(|(_,c)|*c)).flatten();
        if let Some(c)=cmd{self.editor_dimensions_command(c);}
        true
    }
    pub(crate) fn editor_dimensions_input(&mut self,code:Option<KeyCode>,text:Option<&str>,pressed:bool)->bool {
        if self.menus.game_menu.is_some(){return false;}
        let Some(ed)=self.menus.editor.as_mut().filter(|ed|visible(ed)) else{return false;};
        let Some(w)=ed.dimensions_window.as_mut() else{return false;};
        if matches!(code,Some(KeyCode::KeyW|KeyCode::KeyA|KeyCode::KeyS|KeyCode::KeyD|KeyCode::KeyQ|KeyCode::KeyE)) {w.commit();w.input=None;return false;}
        if w.input.is_none(){if pressed && ed.placing_asset.is_some() && matches!(code,Some(KeyCode::KeyN|KeyCode::KeyM)){w.heading+=if code==Some(KeyCode::KeyN){-5.0}else{5.0};return true;}return false;}
        if let Some(k)=code {self.input.keys.remove(&k);}
        if !pressed{return true;}
        if code==Some(KeyCode::Escape){w.input=None;return true;}
        if code==Some(KeyCode::Enter){if w.commit() && ed.splines.dimensions.is_some(){ed.splines.dimensions=Some(w.values);}return true;}
        let (_,value,replace)=w.input.as_mut().unwrap();
        if matches!(code,Some(KeyCode::Backspace|KeyCode::Delete)) {
            if *replace || code==Some(KeyCode::Delete){value.clear();}else{value.pop();}*replace=false;
        } else if let Some(text)=text {
            let t:String=text.chars().filter(|c|c.is_ascii_digit()||matches!(c,'-'|'+'|','|'.')).collect();
            if !t.is_empty(){if *replace{value.clear();*replace=false;}if value.len()+t.len()<=24{value.push_str(&t);}}
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_values_and_accepts_comma() {
        for v in [[f64::NAN,0.0,0.0,0.0],[0.0,0.0,0.0,0.0],[5.0,0.1,0.0,0.0],[5.0,0.0,21.0,0.0]]{assert!(validate(v).is_err());}
        let mut w=Window::default();w.input=Some((0,"5,25".into(),true));assert!(w.commit());assert_eq!(w.values[0],5.25);
        w.input=Some((4,"90".into(),true));assert!(w.commit());assert!((w.value(4)-90.0).abs()<1e-9);assert!(w.values[1]>0.0);
        w.input=Some((0,"-1".into(),true));assert!(!w.commit());assert_eq!(w.values[0],5.25);
    }
}

impl App {
    pub(crate) fn spline_preview_proposal(&mut self)->Option<omsi_map::MapSpline> {
        if self.menus.game_menu.is_some(){return None;}
        let (Some(ed),Some(world),Some(cam),Some(surface))=(self.menus.editor.as_ref(),self.world.as_ref(),self.camera.as_ref(),self.gfx.surface.as_ref())else{return None;};
        if !visible(ed){return None;}let w=ed.dimensions_window.as_ref()?;
        let values=w.live().ok()?;let heading=w.heading;
        let placing=ed.splines.dimensions.is_some() && ed.placing_asset.as_ref().is_some_and(|a|a.kind==crate::asset_catalog::Kind::Spline);
        let (file,at,heading)=if placing {
            let file=ed.placing_asset.as_ref()?.file.clone();
            let on_ui=w.rect.is_some_and(|r|crate::audit_events::contains(r,self.input.cursor)) || self.ui.as_ref().is_some_and(|ui|ui.editor_panel_contains(self.input.cursor));
            let at=if on_ui {w.anchor?}else{let(o,d)=self.world_cursor_ray(cam,(surface.config.width,surface.config.height));crate::placing::ground_hit(world,o,d.as_dvec3(),2000.0)?};
            (file,at,heading)
        }else{let key=ed.splines.selected?;let s=world.spline_edits.lock().current(key)?;let c=omsi_geometry::SplineCurve::from_map(&s,glam::DVec2::new(key.0.0 as f64,key.0.1 as f64)*omsi_map::tile_size());(s.file,c.start,s.heading)};
        let ed=self.menus.editor.as_mut()?;let w=ed.dimensions_window.as_mut()?;w.anchor=Some(at);
        let source=omsi_map::MapSpline{file,pos:at.to_array(),heading,length:values[0],..Default::default()};reshape(&source,values).ok()
    }
    pub(crate) fn spline_preview_frame(&mut self) {
        let proposal=self.spline_preview_proposal();
        if let(Some(world),Some(r),Some(scene))=(self.world.as_ref(),self.renderer.as_ref(),self.scene.as_mut()){omsi_scene::spline_preview::draw(world,r,scene,proposal);}
    }
    pub(crate) fn spline_preview_click(&mut self)->bool {
        if !self.menus.editor.as_ref().is_some_and(|ed|ed.dimensions_window.is_some()&&ed.splines.dimensions.is_some()&&ed.placing_asset.as_ref().is_some_and(|a|a.kind==crate::asset_catalog::Kind::Spline)){return false;}
        let Some(s)=self.spline_preview_proposal()else{self.service_msg=Some(("Check spline values and point at terrain".into(),4.0));return true;};
        let ed=self.menus.editor.as_mut().unwrap();let world=self.world.as_ref().unwrap();let at=glam::DVec3::from(s.pos);
        let message=ed.splines.add_dimensions(world,&s.file,at,at+omsi_geometry::SplineCurve::dir(s.heading).extend(0.0),[s.length,s.radius,s.grad_start,s.pos[2]]);
        self.service_msg=Some((message,5.0));self.editor_reload_splines();true
    }
}

fn close(ed:&mut Editor) {if let Some(w)=ed.dimensions_window.as_mut(){w.commit();}ed.placing_asset=None;ed.splines.cancel_generation();ed.splines.dimensions=None;ed.dimensions_window=None;}
#[cfg(test)]
mod preview_tests {
    use super::*;
    #[test]
    fn live_angle_edits_and_invalid_close_never_trap_camera() {
        let mut w=Window::default();w.values=[20.0,0.0,0.0,10.0];w.input=Some((4,"90".into(),true));assert!(w.commit());
        w.input=Some((0,"150".into(),true));let v=w.live().unwrap();assert!((omsi_geometry::spline_dimensions::turn_degrees(v)-90.0).abs()<1e-8);assert_eq!(w.values[0],20.0);
        w.input=Some((4,"675".into(),true));assert!(w.commit());w.input=Some((0,"-".into(),true));assert!(w.live().is_err());
        let mut ed=Editor::default();ed.dimensions_window=Some(w);ed.splines.dimensions=Some([20.0,0.0,0.0,10.0]);close(&mut ed);
        assert!(ed.dimensions_window.is_none()&&ed.placing_asset.is_none()&&ed.splines.dimensions.is_none());
        let keys=[KeyCode::KeyW,KeyCode::KeyQ].into_iter().collect();assert!(crate::editor::camera_key_pressed(&keys,Some(&ed),KeyCode::KeyW));
    }
}
