//! Numeric spline dimensions. Presets apply to newly placed splines; existing roads
//! change only after Apply, through the editor's normal undo/save path.
use crate::{app::App, editor::Editor};
use winit::keyboard::KeyCode;

#[derive(Clone, Copy)]
pub enum Command { Edit(usize), Read, Apply, Preset, Close }
pub struct Window {
    pub values: [f64;4],
    pub input: Option<(usize,String,bool)>,
    pub rect: Option<[f32;4]>,
    pub buttons: Vec<([f32;4],Command)>,
    pub message: String,
}
impl Default for Window {
    fn default()->Self { Self { values:[5.0,0.0,0.0,0.0],input:None,rect:None,buttons:Vec::new(),message:String::new() } }
}
pub fn visible(ed:&Editor)->bool {
    ed.spline_mode && ed.catalog.is_none() && ed.junction_window.is_none() && ed.construction_window.is_none()
        && ed.sidewalk_window.is_none() && ed.roadside_window.is_none() && ed.traffic_window.is_none()
        && ed.tile_window.is_none() && ed.text_window.is_none() && !ed.terrain.active
}
pub use omsi_geometry::spline_dimensions::{validate, reshape};
impl Window {
    fn commit(&mut self)->bool {
        let Some((i,text,_))=&self.input else{return true;};
        let Ok(n)=text.replace(',',".").parse::<f64>() else {self.message="Enter a valid number".into();return false;};
        let mut v=self.values;v[*i]=n;
        if let Err(e)=validate(v){self.message=e;return false;}
        self.values=v;self.input=None;true
    }
}
impl App {
    pub(crate) fn editor_open_dimensions(&mut self) {
        if self.net.lan.is_some(){return;}
        let (Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref()) else{return;};
        ed.splines.finish_drag();
        let mut w=Window::default();
        if let Some(s)=ed.splines.selected.and_then(|k|world.spline_edits.lock().current(k)) {
            w.values=[s.length,s.radius,s.grad_start,s.pos[2]];
        } else if let Some(v)=ed.splines.dimensions {w.values=v;}
        w.message="Radius 0 = straight · Height = absolute start height".into();
        ed.dimensions_window=Some(w);
    }
    fn editor_dimensions_command(&mut self,c:Command) {
        let (Some(ed),Some(world))=(self.menus.editor.as_mut(),self.world.as_ref()) else{return;};
        if matches!(c,Command::Close){ed.dimensions_window=None;return;}
        let Some(w)=ed.dimensions_window.as_mut() else{return;};
        if !w.commit(){return;}
        match c {
            Command::Edit(i)=>{w.input=Some((i,format!("{:.4}",w.values[i]),true));self.input.keys.clear();}
            Command::Read=>{
                if let Some(s)=ed.splines.selected.and_then(|k|world.spline_edits.lock().current(k)) {
                    w.values=[s.length,s.radius,s.grad_start,s.pos[2]];w.message="Selected spline values loaded".into();
                } else {w.message="Select a road first".into();}
            }
            Command::Apply=>{
                w.message=match ed.splines.apply_dimensions(world,w.values){Ok(())=>"Spline dimensions applied · Ctrl+Z undo · Ctrl+S save".into(),Err(e)=>e};
            }
            Command::Preset=>{
                if ed.splines.dimensions.is_some(){ed.splines.dimensions=None;}
                else if let Err(e)=validate(w.values){w.message=e;return;}else{ed.splines.dimensions=Some(w.values);}
                w.message="New spline: choose a type, click start, then direction".into();
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
        if w.input.is_none(){return false;}
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
        w.input=Some((0,"-1".into(),true));assert!(!w.commit());assert_eq!(w.values[0],5.25);
    }
}
