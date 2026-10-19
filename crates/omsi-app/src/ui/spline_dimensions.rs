use super::*;
impl Ui {
    pub fn draw_spline_dimensions(&mut self,r:&Renderer,scene:&mut Scene,editor:Option<&mut crate::editor::Editor>,scale:f32,viewport:[f32;4],cursor:(f32,f32)) {
        use crate::spline_dimensions::Command;
        let Some(ed)=editor.filter(|ed|crate::spline_dimensions::visible(ed)) else{return;};
        let Some(w)=ed.dimensions_window.as_mut() else{return;};
        let s=scale.max(0.5).min(viewport[2]/360.0).min(viewport[3]/407.0);
        let(x,y)=self.editor_window_origin("spline-dimensions",viewport,[350.0*s,397.0*s],[viewport[0]+viewport[2]-362.0*s,viewport[1]+110.0*s],[350.0*s,32.0*s]);
        let rect=|a:f32,b:f32,c:f32,d:f32|[x+a*s,y+b*s,x+(a+c)*s,y+(b+d)*s];
        let flat=self.text.flat;self.text.flat=true;w.buttons.clear();w.rect=Some(rect(0.0,0.0,350.0,397.0));
        self.editor_surface(r,scene,w.rect.unwrap(),5.0*s,PANEL);
        self.catalog_text(r,scene,"Spline dimensions · drag here",(x+10.0*s,y+9.0*s),330.0*s,13.0*s,WHITE);
        let mut buttons=Vec::new();
        for(row,(i,label))in [(0,"Length (m)"),(1,"Radius (m)"),(4,"Curve angle (degrees)"),(2,"Gradient (%)"),(3,"Start height (m)")].into_iter().enumerate(){
            let at=43.0+row as f32*37.0;
            self.catalog_text(r,scene,label,(x+12.0*s,y+(at+5.0)*s),180.0*s,12.0*s,WHITE);
            let value=w.input.as_ref().filter(|(f,_,_)|*f==i).map(|(_,t,_)|t.clone()).unwrap_or_else(||format!("{:.3}",w.value(i)));
            buttons.push((rect(198.0,at,140.0,29.0),value,Command::Edit(i)));
        }
        buttons.extend([
            (rect(12.0,230.0,158.0,29.0),"Read selected spline".into(),Command::Read),
            (rect(180.0,230.0,158.0,29.0),"Apply to selected spline".into(),Command::Apply),
            (rect(12.0,266.0,326.0,29.0),if ed.splines.dimensions.is_some(){"New spline dimensions: ON"}else{"New spline dimensions: OFF"}.into(),Command::Preset),
            (rect(228.0,357.0,110.0,28.0),"Close".into(),Command::Close)]);
        for(rect,label,command)in buttons{
            self.editor_surface(r,scene,rect,3.0*s,if editor_contains(rect,cursor){LIT}else{CHIP});
            self.catalog_text(r,scene,&label,(rect[0]+6.0*s,rect[1]+6.0*s),rect[2]-rect[0]-12.0*s,11.0*s,WHITE);
            w.buttons.push((rect,command));
        }
        for(i,line)in wrap(&self.text,&omsi_ui::tr(&w.message),10.0*s,326.0*s).iter().take(3).enumerate(){
            self.catalog_text(r,scene,line,(x+12.0*s,y+(304.0+i as f32*15.0)*s),326.0*s,10.0*s,MUTED);
        }
        self.catalog_text(r,scene,"Constant gradient; check linked ends",(x+12.0*s,y+362.0*s),208.0*s,10.0*s,MUTED);
        self.text.flat=flat;
    }
}
