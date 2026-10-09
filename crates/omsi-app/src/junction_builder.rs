//! Parameterised T/X junctions. Mesh, markings and AI paths come from the same arms.
use glam::{DVec2,DVec3,Mat4,Vec2,Vec3};
use omsi_o3d::{Material,Mesh,Triangle,Vertex};
use serde::{Deserialize,Serialize};
use std::path::{Path,PathBuf};

#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Arm {pub enabled:bool,pub angle:f64,pub width:f64,pub length:f64,pub bend:f64,pub sidewalk:f64}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct RoadSurface {pub u_start:f64,pub u_end:f64,pub reverse_v:bool,pub width_metres:f64}
impl Default for RoadSurface {fn default()->Self {Self {u_start:0.0,u_end:1.0,reverse_v:false,width_metres:4.0}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Project {pub format:u32,pub name:String,pub arms:[Arm;4],pub corner:f64,pub texture_metres:f64,
    #[serde(default)] pub road_surface:Option<RoadSurface>,
    pub road_texture:String,pub walk_texture:String,pub markings:bool,pub left_hand:bool}

impl Default for Project {
    fn default()->Self {Self {format:1,name:"Eigene T-Kreuzung".into(),arms:std::array::from_fn(|i|Arm {
        enabled:i!=3,angle:[270.0,90.0,180.0,0.0][i],width:7.0,length:30.0,bend:0.0,sidewalk:0.0}),
        corner:5.0,texture_metres:4.0,road_surface:Some(RoadSurface::default()),road_texture:String::new(),walk_texture:String::new(),markings:true,left_hand:false}}
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Field {Name,Width,Length,Angle,Bend,Sidewalk,Corner,TextureMetres,UStart,UEnd,TextureWidth}
impl Field {
    pub fn title(self)->&'static str {match self {Self::Name=>"Name",Self::Width=>"Fahrbahnbreite (m)",Self::Length=>"Länge ab Mitte (m)",
        Self::Angle=>"Richtung (°)",Self::Bend=>"Biegung am Ende (°)",Self::Sidewalk=>"Gehweg je Seite (m)",Self::Corner=>"Eckrundung (m)",Self::TextureWidth=>"Texturbreite (m)",Self::UStart=>"Ausschnitt U von",Self::UEnd=>"Ausschnitt U bis",Self::TextureMetres=>"Texturlänge (m)"}}
    pub fn texture_index(self)->Option<usize> {match self {Self::UStart=>Some(0),Self::UEnd=>Some(1),Self::TextureWidth=>Some(2),_=>None}}
    fn limits(self)->(f64,f64) {match self {Self::Width=>(4.0,30.0),Self::Length=>(8.0,150.0),Self::Angle=>(0.0,360.0),
        Self::Bend=>(-35.0,35.0),Self::Sidewalk=>(0.0,6.0),Self::Corner=>(0.0,20.0),Self::TextureMetres=>(0.25,100.0),Self::TextureWidth=>(0.25,100.0),Self::UStart|Self::UEnd=>(-16.0,16.0),Self::Name=>(0.0,0.0)}}
}
#[derive(Clone,Copy)]
pub enum Command {Close,Shape(bool),Arm(usize),Edit(Field),Digit(Field,i32),Step(Field,f64),Adjust(Field,f64),RoadTexture,WalkTexture,Markings,
    ShowPaths,Rotate,Undo,Redo,Export,Load,UseSpline,ConnectRoad}

pub struct Input {pub field:Field,pub text:String,pub replace:bool}
pub struct Window {
    pub texture_digits:[usize;3],pub texture_steps:[i32;3],pub texture_focus:Option<Field>,
    pub existing_preview:Vec<[DVec3;2]>,pub arm_links:[Option<i64>;4],pub source_label:String,pub road_preview:Vec<[DVec3;2]>,pub pending:Option<(((i32,i32),i64),usize)>,pub target:Option<i64>,pub placed_project:Option<Project>,pub project:Project,pub arm:usize,pub input:Option<Input>,pub message:String,pub rects:Vec<([f32;4],Command)>,
    pub preview:Option<omsi_texture::Image>,pub preview_id:u64,pub show_paths:bool,pub view:u8,
    pub error:Option<String>,pub undo:Vec<Project>,pub redo:Vec<Project>,dirty:bool,
}

impl Window {
    pub fn new(left_hand:bool)->Self {let project=Project {left_hand,..Default::default()};Self {texture_digits:[4;3],texture_steps:[-4;3],texture_focus:None,existing_preview:Vec::new(),arm_links:[None;4],source_label:String::new(),road_preview:Vec::new(),pending:None,target:None,placed_project:None,project,arm:0,input:None,
        message:"Arm wählen, Maße einstellen, dann Speichern & einsetzen. Blau = KI-Wege.".into(),rects:Vec::new(),preview:None,
        preview_id:0,show_paths:true,view:0,error:None,undo:Vec::new(),redo:Vec::new(),dirty:true}}
    pub fn hit(&self,p:(f32,f32))->Option<Command> {self.rects.iter().rev().find(|(r,_)|
        p.0>=r[0]&&p.0<=r[2]&&p.1>=r[1]&&p.1<=r[3]).map(|(_,c)|*c)}
    pub fn value(&self,field:Field)->f64 {let arm=&self.project.arms[self.arm];match field {
        Field::Width=>arm.width,Field::Length=>arm.length,Field::Angle=>arm.angle,Field::Bend=>arm.bend,Field::Sidewalk=>arm.sidewalk,
        Field::TextureWidth=>self.project.road_surface.as_ref().map_or(self.project.texture_metres,|p|p.width_metres),
        Field::UStart=>self.project.road_surface.as_ref().map_or(0.0,|p|p.u_start),
        Field::UEnd=>self.project.road_surface.as_ref().map_or(1.0,|p|p.u_end),Field::Corner=>self.project.corner,Field::TextureMetres=>self.project.texture_metres,Field::Name=>0.0}}
    pub fn edit(&mut self,field:Field) {self.input=Some(Input {field,text:if field==Field::Name {self.project.name.clone()}
        else {self.number_text(field)},replace:true});}
    pub fn display_digits(&self,field:Field)->usize {
        let Some(i)=field.texture_index() else {return 2;};
        let mut digits=self.texture_digits[i];
        // Undo and imported profiles may restore more precision than the last typed value.
        while digits<4 {
            let scale=10_f64.powi(digits as i32);
            if ((self.value(field)*scale).round()/scale-self.value(field)).abs()<1e-8 {break;}
            digits+=1;
        }
        digits
    }
    pub fn number_text(&self,field:Field)->String {
        format!("{:.*}",self.display_digits(field),self.value(field))
    }
    pub fn texture_step(&self,field:Field)->f64 {
        field.texture_index().map_or(1.0,|i|10_f64.powi(self.texture_steps[i]))
    }
    pub fn step_texture(&mut self,field:Field,direction:f64) {
        let Some(i)=field.texture_index() else {return;};
        if !self.commit() {return;}
        let step=self.texture_step(field);
        // Preserve the other decimal places when stepping the selected digit.
        let precision=10_f64.powi(self.display_digits(field) as i32);
        self.set(field,((self.value(field)+direction*step)*precision).round()/precision);
        // Bounds such as 0.25 m must remain visible after a whole-number entry.
        while self.texture_digits[i]<4 && (self.number_text(field).parse::<f64>().unwrap_or(0.0)-self.value(field)).abs()>1e-8 {
            self.texture_digits[i]+=1;
        }
        self.texture_focus=Some(field);
    }
    fn remember(&mut self) {self.undo.push(self.project.clone());if self.undo.len()>128 {self.undo.remove(0);}self.redo.clear();}
    pub fn clear_connection_preview(&mut self){self.changed();}
    fn changed(&mut self) {self.dirty=true;self.error=None;self.road_preview.clear();self.pending=None;}
    pub fn connection_preview(&mut self,points:Vec<[DVec3;2]>,existing:Vec<[DVec3;2]>,links:[Option<i64>;4]){self.changed();self.road_preview=points;self.existing_preview=existing;self.arm_links=links;}
    pub fn set(&mut self,field:Field,value:f64) {let (lo,hi)=field.limits();let value=value.clamp(lo,hi);
        if !value.is_finite() || field==Field::Name || (self.value(field)-value).abs()<1e-8 {return;}
        self.remember();let arm=&mut self.project.arms[self.arm];match field {Field::Width=>arm.width=value,Field::Length=>arm.length=value,
            Field::Angle=>arm.angle=value,Field::Bend=>arm.bend=value,Field::Sidewalk=>arm.sidewalk=value,
            Field::TextureWidth=>self.project.road_surface.get_or_insert_with(RoadSurface::default).width_metres=value,
            Field::UStart=>self.project.road_surface.get_or_insert_with(RoadSurface::default).u_start=value,
            Field::UEnd=>self.project.road_surface.get_or_insert_with(RoadSurface::default).u_end=value,Field::Corner=>self.project.corner=value,Field::TextureMetres=>self.project.texture_metres=value,Field::Name=>{}}
        self.changed();
    }
    pub fn commit(&mut self)->bool {
        let Some(input)=self.input.as_ref() else {return true;};let field=input.field;let text=input.text.trim().to_string();
        if field==Field::Name {
            if text.is_empty() || text.chars().count()>80 || text.chars().any(char::is_control) {self.message="Name benötigt 1–80 Zeichen".into();return false;}
            if self.project.name!=text {self.remember();self.project.name=text;self.changed();}
        } else {let (lo,hi)=field.limits();match crate::terrain_editor::parse_number(&text,lo,hi) {
            Ok(v)=>{
                if let Some(i)=field.texture_index() {
                    if !input.replace {
                        let digits=text.split(['.',',']).nth(1).map_or(0,str::len);
                        if digits>4 {self.message="Bitte höchstens 4 Nachkommastellen eingeben".into();return false;}
                        self.texture_digits[i]=digits;self.texture_steps[i]=-(digits as i32);
                    }
                }
                if !input.replace {self.set(field,v);}
            },Err(e)=>{self.message=e;return false;}}}
        self.input=None;true
    }
    pub fn command(&mut self,command:Command) {
        match command {
            Command::Step(f,d)=>self.step_texture(f,d),
            Command::Digit(f,power)=>{
                if let Some(i)=f.texture_index() {if self.commit() {
                    self.texture_steps[i]=power.clamp(-(self.display_digits(f) as i32),2);
                    self.texture_focus=Some(f);self.edit(f);
                }}
            },
            Command::Edit(f)=>{if self.commit() {self.edit(f);}},
            Command::Arm(i) if i<4 && self.project.arms[i].enabled=>{if self.commit() {self.arm=i;}},
            Command::Adjust(f,d)=>{if self.commit() {self.set(f,self.value(f)+d);}},
            Command::Shape(cross)=>{if !self.commit() || self.project.arms[3].enabled==cross {return;}self.remember();self.project.arms[3].enabled=cross;
                if matches!(self.project.name.as_str(),"Eigene T-Kreuzung"|"Eigene Vierarmkreuzung") {
                    self.project.name=if cross {"Eigene Vierarmkreuzung"} else {"Eigene T-Kreuzung"}.into();
                }
                if !cross && self.arm==3 {self.arm=2;}self.changed();},
            Command::Markings=>{if !self.commit() {return;}self.remember();self.project.markings=!self.project.markings;self.changed();},
            Command::ShowPaths=>{self.show_paths=!self.show_paths;self.dirty=true;},
            Command::Rotate=>{self.view=(self.view+1)%4;self.dirty=true;},
            Command::Undo=>{self.input=None;if let Some(p)=self.undo.pop() {self.redo.push(self.project.clone());self.project=p;
                if !self.project.arms[self.arm].enabled {self.arm=0;}self.changed();}},
            Command::Redo=>{self.input=None;if let Some(p)=self.redo.pop() {self.undo.push(self.project.clone());self.project=p;
                if !self.project.arms[self.arm].enabled {self.arm=0;}self.changed();}},
            _=>{}
        }
    }
    pub fn texture(&mut self,file:String,walk:bool) {self.remember();if walk {self.project.walk_texture=file;} else {self.project.road_texture=file;self.project.road_surface=Some(RoadSurface::default());}self.changed();}
    pub fn use_spline(&mut self,width:f64,texture:String,surface:RoadSurface,metres:f64) {
        self.remember();self.project.arms[self.arm].width=width;
        self.project.road_texture=texture;self.project.road_surface=Some(surface);self.project.texture_metres=metres;self.changed();
        self.message="Fahrbahntextur und Ausschnitt übernommen. U von/bis prüfen: eingebrannte Linien ggf. aussparen. Gilt für alle Arme.".into();
    }
    pub fn refresh(&mut self,root:&Path) {
        if !self.dirty {return;}self.dirty=false;self.preview_id=self.preview_id.wrapping_add(1);self.preview=None;
        match build(&self.project) {Ok(mut built)=>{
            if !self.existing_preview.is_empty(){let slot=built.mesh.materials.len()as u16;built.mesh.materials.push(Material{diffuse:[0.15,0.85,0.32,1.0],..Default::default()});
                for pair in self.existing_preview.chunks_exact(2){quad(&mut built.mesh,pair[0][0]+DVec3::Z*0.03,pair[0][1]+DVec3::Z*0.03,pair[1][1]+DVec3::Z*0.03,pair[1][0]+DVec3::Z*0.03,slot,4.0);}}
            if !self.road_preview.is_empty(){let slot=built.mesh.materials.len()as u16;built.mesh.materials.push(Material{diffuse:[0.1,0.65,1.0,1.0],..Default::default()});
                for pair in self.road_preview.chunks_exact(2){quad(&mut built.mesh,pair[0][0]+DVec3::Z*0.03,pair[0][1]+DVec3::Z*0.03,pair[1][1]+DVec3::Z*0.03,pair[1][0]+DVec3::Z*0.03,slot,4.0);}}
            if self.show_paths {path_overlay(&mut built.mesh,&built.paths);}
            match crate::asset_catalog::junction_preview(root,&built.mesh,self.view,self.project.road_surface.as_ref()) {
                Ok(img)=>{self.preview=Some(img);self.error=None;},Err(e)=>self.error=Some(e)}
        },Err(e)=>{self.error=Some(e.clone());self.message=e;}}
    }
    pub fn load(&mut self,path:&Path)->Result<(),String> {
        let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
        if bytes.len()>64*1024 {return Err("Kreuzungsprojekt ist zu groß".into());}
        let project:Project=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;build(&project)?;
        self.remember();self.project=project;if !self.project.arms[self.arm].enabled {self.arm=0;}self.input=None;self.changed();
        self.message="Projekt geladen · Änderungen werden beim Export als neues Bauteil gespeichert".into();Ok(())
    }
}

fn validate_surface(surface:&RoadSurface)->Result<(),String> {
    if !surface.width_metres.is_finite() || !(0.25..=100.0).contains(&surface.width_metres) {return Err("Texturbreite benötigt 0,25–100 m".into());}
    if !surface.u_start.is_finite() || !surface.u_end.is_finite() || surface.u_start.abs()>16.0 || surface.u_end.abs()>16.0
        || (surface.u_end-surface.u_start).abs()<0.0001 {return Err("Texturausschnitt U von/bis muss verschieden sein und zwischen -16 und 16 liegen".into());}Ok(())
}
/// Materialise the horizontal atlas interval so wrapping cannot sample adjacent markings.
/// The same pixels are used in the preview and the exported native OMSI texture.
pub fn crop_surface(image:&omsi_texture::Image,surface:&RoadSurface)->Result<omsi_texture::Image,String> {
    validate_surface(surface)?;
    if image.width==0 || image.height==0 || image.rgba.len()!=image.width as usize*image.height as usize*4 {return Err("Ungültige Bilddaten".into());}
    let width=((surface.u_end-surface.u_start).abs()*image.width as f64).round().clamp(1.0,4096.0) as u32;
    let mut rgba=Vec::with_capacity(width as usize*image.height as usize*4);
    for y in 0..image.height {for x in 0..width {
        let u=surface.u_start+(surface.u_end-surface.u_start)*(x as f64+0.5)/width as f64;
        let sx=(u.rem_euclid(1.0)*image.width as f64).floor() as u32;
        let i=((y*image.width+sx.min(image.width-1))*4) as usize;rgba.extend_from_slice(&image.rgba[i..i+4]);
    }}
    Ok(omsi_texture::Image {width,height:image.height,rgba,has_alpha:image.has_alpha})
}

/// Pick the widest opaque, nearly horizontal profile segment overlapping a driving path.
/// This avoids assuming slot zero is asphalt (it may be a kerb, verge or decal).
pub fn spline_surface(def:&omsi_scenery::Spline,mirror:bool,length:f64)->Result<(usize,RoadSurface,f64),String> {
    let paths:Vec<_>=def.paths.iter().filter(|p|p.kind==0).collect();
    if paths.is_empty() {return Err("Spline besitzt keine Fahrbahnwege".into());}
    let road_lo=paths.iter().map(|p|p.start[0] as f64-p.width as f64/2.0).fold(f64::INFINITY,f64::min);
    let road_hi=paths.iter().map(|p|p.start[0] as f64+p.width as f64/2.0).fold(f64::NEG_INFINITY,f64::max);
    let mut candidates=Vec::new();
    for profile in &def.profiles {
        let Some(texture)=def.textures.get(profile.texture) else {continue;};
        if texture.alpha!=0 {continue;}
        for pair in profile.points.windows(2) {
            let (a,b)=(&pair[0],&pair[1]);
            if (a.x-b.x).abs()<0.01 || (a.z-b.z).abs()>0.05 {continue;}
            {
                let lo=(a.x.min(b.x) as f64).max(road_lo);
                let hi=(a.x.max(b.x) as f64).min(road_hi);
                if hi-lo<0.25 {continue;}
                candidates.push((hi-lo,profile.texture,a,b,lo,hi));
            }
        }
    }
    let (_,slot,a,b,lo,hi)=candidates.into_iter().max_by(|a,b|a.0.total_cmp(&b.0)).ok_or("Keine eindeutige Fahrbahnfläche; Asphalt bitte im Texturkatalog auswählen")?;
    let texture=&def.textures[slot];
    if texture.patchwork.is_some() || (a.v_scale-b.v_scale).abs()>0.00001 {return Err("Patchwork oder variable Texturskalierung: Asphalt bitte manuell auswählen".into());}
    let u=|x:f64|a.u as f64+(b.u-a.u) as f64*(x-a.x as f64)/(b.x-a.x) as f64;
    let scale=a.v_scale as f64/if texture.scale_by_length {length} else {1.0};
    let metres=1.0/scale.abs();
    if !metres.is_finite() || !(0.25..=100.0).contains(&metres) {return Err("Texturlänge außerhalb von 0,25–100 m; Textur manuell auswählen".into());}
    let (u_start,u_end)=if mirror {(u(hi),u(lo))} else {(u(lo),u(hi))};
    let surface=RoadSurface {u_start,u_end,reverse_v:(scale<0.0)^mirror,width_metres:hi-lo};validate_surface(&surface)?;
    Ok((slot,surface,metres))
}

pub struct Built {pub mesh:Mesh,pub paths:Vec<(Vec<DVec3>,f64,i32)>,pub outline:Vec<DVec2>}
fn direction(angle:f64)->DVec2 {let (s,c)=angle.to_radians().sin_cos();DVec2::new(s,c)}
fn right(v:DVec2)->DVec2 {DVec2::new(v.y,-v.x)}
fn point(arm:&Arm,core:f64,s:f64,offset:f64,z:f64)->DVec3 {
    let theta=arm.angle.to_radians();let k=arm.bend.to_radians()/(arm.length-core);
    let d=direction(arm.angle);let xy=if k.abs()<1e-10 {d*(core+s)} else {
        d*core+DVec2::new((theta.cos()-(theta+k*s).cos())/k,((theta+k*s).sin()-theta.sin())/k)
    }+right(direction(arm.angle+arm.bend*s/(arm.length-core)))*offset;
    DVec3::new(xy.x,xy.y,z)
}
/// Exact centre, outward heading and cross-section of an exported arm.
pub fn port(project:&Project,index:usize)->Result<(DVec3,f64,omsi_scenery::Spline),String>{
    build(project)?;
    let a=project.arms.get(index).filter(|a|a.enabled).ok_or("Dieser Kreuzungsarm ist nicht aktiv")?;
    let core=project.arms.iter().filter(|a|a.enabled).map(|a|a.width/2.0+a.sidewalk).fold(0.0,f64::max)+project.corner+1.0;
    let mut points=Vec::new();let half=a.width/2.0;
    if a.sidewalk>0.0 {points.extend([(-half-a.sidewalk,0.15),(-half,0.15)]);}
    points.extend([(-half,0.0),(half,0.0)]);
    if a.sidewalk>0.0 {points.extend([(half,0.15),(half+a.sidewalk,0.15)]);}
    let def=omsi_scenery::Spline{profiles:vec![omsi_scenery::SplineProfile{texture:0,points:points.into_iter().map(|(x,z)|omsi_scenery::SplineProfilePoint{x:x as f32,z:z as f32,u:0.0,v_scale:1.0}).collect()}],..Default::default()};
    Ok((point(a,core,a.length-core,0.0,0.0),a.angle+a.bend,def))
}

fn quadratic(a:DVec2,b:DVec2,c:DVec2,t:f64)->DVec2 {let u=1.0-t;a*u*u+b*2.0*u*t+c*t*t}
fn cubic(a:DVec3,b:DVec3,c:DVec3,d:DVec3,t:f64)->DVec3 {let u=1.0-t;a*u*u*u+b*3.0*u*u*t+c*3.0*u*t*t+d*t*t*t}
fn corner(a:&Arm,b:&Arm,core:f64)->Vec<DVec2> {
    let p=point(a,core,0.0,a.width/2.0,0.0).truncate();let q=point(b,core,0.0,-b.width/2.0,0.0).truncate();
    let da=-direction(a.angle);let db=-direction(b.angle);let det=da.perp_dot(db);
    if det.abs()<1e-6 {return vec![p,q];}
    let s=(q-p).perp_dot(db)/det;let t=(q-p).perp_dot(da)/det;
    if s<0.0 || t<0.0 || s>core*3.0 || t>core*3.0 {return vec![p,q];}
    let c=p+da*s;(0..=12).map(|i|quadratic(p,c,q,i as f64/12.0)).collect()
}

fn vertex(mesh:&mut Mesh,p:DVec3,normal:DVec3,scale:f64)->u32 {
    let index=mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex {position:Vec3::new(p.x as f32,p.z as f32,p.y as f32),normal:Vec3::new(normal.x as f32,normal.z as f32,normal.y as f32),
        uv:Vec2::new((p.x/scale) as f32,(p.y/scale) as f32)});index
}
fn triangle(mesh:&mut Mesh,a:DVec3,b:DVec3,c:DVec3,slot:u16,scale:f64) {
    let mut points=[a,b,c];if (c-a).cross(b-a).z<0.0 {points.swap(1,2);}
    let normal=(points[2]-points[0]).cross(points[1]-points[0]).normalize_or_zero();
    if normal.length_squared()<0.5 {return;}
    let indices=points.map(|p|vertex(mesh,p,normal,scale));mesh.triangles.push(Triangle {indices,material:slot});
}
fn quad(mesh:&mut Mesh,a:DVec3,b:DVec3,c:DVec3,d:DVec3,slot:u16,scale:f64) {
    triangle(mesh,a,b,c,slot,scale);triangle(mesh,a,c,d,slot,scale);
}
fn ribbon(mesh:&mut Mesh,points:&[(DVec3,DVec3)],slot:u16,scale:f64) {
    for w in points.windows(2) {quad(mesh,w[0].0,w[1].0,w[1].1,w[0].1,slot,scale);}
}
fn contains(p:DVec2,outline:&[DVec2])->bool {
    let mut inside=false;
    for i in 0..outline.len() {let (a,b)=(outline[i],outline[(i+1)%outline.len()]);
        let ab=b-a;let t=(p-a).dot(ab)/ab.length_squared().max(1e-12);
        if (p-(a+ab*t.clamp(0.0,1.0))).length()<0.01 {return true;}
        if (a.y>p.y)!=(b.y>p.y) && p.x<(b.x-a.x)*(p.y-a.y)/(b.y-a.y)+a.x {inside=!inside;}
    }inside
}

pub fn build(project:&Project)->Result<Built,String> {
    if project.format!=1 {return Err("Unbekanntes Kreuzungsprojektformat".into());}
    if project.name.trim().is_empty() || project.name.chars().count()>80 || project.name.chars().any(char::is_control) {
        return Err("Kreuzung benötigt einen Namen mit 1–80 Zeichen".into());
    }
    for (f,v) in [(Field::Corner,project.corner),(Field::TextureMetres,project.texture_metres)] {
        let (lo,hi)=f.limits();if !v.is_finite() || v<lo || v>hi {return Err(format!("Ungültiger Wert: {}",f.title()));}
    }
    if let Some(surface)=&project.road_surface {validate_surface(surface)?;}
    for texture in [&project.road_texture,&project.walk_texture] {
        let normal=texture.replace('\\',"/");let path=Path::new(&normal);
        if !texture.is_empty() && (path.is_absolute() || normal.contains(':') || normal.chars().any(char::is_control)
            || path.components().any(|c|!matches!(c,std::path::Component::Normal(_)))) {
            return Err("Textur benötigt einen relativen OMSI-Inhaltepfad".into());
        }
    }
    let mut arms:Vec<_>=project.arms.iter().filter(|a|a.enabled).collect();
    if !(3..=4).contains(&arms.len()) {return Err("Drei oder vier Straßenarme auswählen".into());}
    for a in &arms {for (f,v) in [(Field::Width,a.width),(Field::Length,a.length),(Field::Angle,a.angle),(Field::Bend,a.bend),(Field::Sidewalk,a.sidewalk)] {
        let (lo,hi)=f.limits();if !v.is_finite() || v<lo || v>hi {return Err(format!("Ungültiger Armwert: {}",f.title()));}
    }}
    arms.sort_by(|a,b|a.angle.rem_euclid(360.0).total_cmp(&b.angle.rem_euclid(360.0)));
    for i in 0..arms.len() {let gap=(arms[(i+1)%arms.len()].angle-arms[i].angle).rem_euclid(360.0);
        if !(35.0..=180.00001).contains(&gap) {return Err("Straßenarme benötigen 35–180° Abstand; Hauptarme gegebenenfalls gegenüberstellen".into());}}
    let core=arms.iter().map(|a|a.width/2.0+a.sidewalk).fold(0.0,f64::max)+project.corner+1.0;
    for a in &arms {
        if a.length<core+4.0 {return Err(format!("Arme müssen mindestens {:.1} m lang sein; Eckrundung oder Breite verkleinern",core+4.0));}
        if a.bend.abs()>0.001 && (a.length-core)/a.bend.to_radians().abs()<=a.width/2.0+a.sidewalk+1.0 {
            return Err("Biegung ist für diese Straßenbreite zu eng".into());
        }
    }
    let mut mesh=Mesh {version:4,transform:Mat4::IDENTITY,has_transform:true,materials:vec![
        Material {texture:project.road_texture.clone(),diffuse:if project.road_texture.is_empty() {[0.22,0.23,0.24,1.0]} else {[1.0;4]},..Default::default()},
        Material {texture:project.walk_texture.clone(),diffuse:if project.walk_texture.is_empty() {[0.65,0.65,0.62,1.0]} else {[1.0;4]},..Default::default()},
        Material {diffuse:[0.96,0.94,0.84,1.0],..Default::default()}],..Default::default()};
    let mut outline=Vec::new();let mut paths=Vec::new();let sign=if project.left_hand {-1.0} else {1.0};
    for (i,arm) in arms.iter().enumerate() {
        let n=((arm.length-core)/0.75).ceil() as usize;let ds=(arm.length-core)/n as f64;
        let stations:Vec<_>=(0..=n).map(|j|j as f64*ds).collect();
        let strip:Vec<_>=stations.iter().map(|&s|(point(arm,core,s,-arm.width/2.0,0.0),point(arm,core,s,arm.width/2.0,0.0))).collect();
        let start=mesh.vertices.len();
        ribbon(&mut mesh,&strip,0,project.texture_metres);
        if let Some(surface)=&project.road_surface {
            // Triangles own their vertices: assign UVs from the exact generating stations.
            for (j,w) in strip.windows(2).enumerate() {
                for v in &mut mesh.vertices[start+j*6..start+(j+1)*6] {
                    let p=DVec3::new(v.position.x as f64,v.position.z as f64,v.position.y as f64);
                    let corners=[w[0].0,w[1].0,w[1].1,w[0].1];
                    let k=(0..4).min_by(|&a,&b|(corners[a]-p).length_squared().total_cmp(&(corners[b]-p).length_squared())).unwrap();
                    let distance=core+stations[j+usize::from(k==1||k==2)];
                    v.uv=Vec2::new(if k>=2 {(arm.width/surface.width_metres) as f32} else {0.0},(distance/project.texture_metres*if surface.reverse_v {-1.0} else {1.0}) as f32);
                }
            }
        }
        outline.extend(strip.iter().map(|p|p.0.truncate()));outline.extend(strip.iter().rev().map(|p|p.1.truncate()));
        if arm.sidewalk>0.0 {for side in [-1.0,1.0] {
            let walk:Vec<_>=stations.iter().map(|&s|(point(arm,core,s,side*arm.width/2.0,0.15),
                point(arm,core,s,side*(arm.width/2.0+arm.sidewalk),0.15))).collect();
            ribbon(&mut mesh,&walk,1,project.texture_metres);
            let kerb:Vec<_>=stations.iter().map(|&s|(point(arm,core,s,side*arm.width/2.0,0.0),point(arm,core,s,side*arm.width/2.0,0.15))).collect();
            ribbon(&mut mesh,&kerb,1,project.texture_metres);
        }}
        if project.markings {for (j,w) in stations.windows(2).enumerate() {if ((j as f64*ds)/4.0).floor() as usize%2==0 {
            quad(&mut mesh,point(arm,core,w[0],-0.06,0.012),point(arm,core,w[1],-0.06,0.012),
                point(arm,core,w[1],0.06,0.012),point(arm,core,w[0],0.06,0.012),2,1.0);
        }}}
        let incoming:Vec<_>=stations.iter().rev().map(|&s|point(arm,core,s,-sign*arm.width/4.0,0.0)).collect();
        let outgoing:Vec<_>=stations.iter().map(|&s|point(arm,core,s,sign*arm.width/4.0,0.0)).collect();
        paths.push((incoming,arm.width/2.0,1));paths.push((outgoing,arm.width/2.0,1));
        let next=arms[(i+1)%arms.len()];let curve=corner(arm,next,core);outline.extend(curve.iter().skip(1).take(curve.len().saturating_sub(2)).copied());
        let centre_start=mesh.vertices.len();
        for w in curve.windows(2) {triangle(&mut mesh,DVec3::ZERO,w[0].extend(0.0),w[1].extend(0.0),0,project.texture_metres);}
        triangle(&mut mesh,DVec3::ZERO,point(arm,core,0.0,-arm.width/2.0,0.0),point(arm,core,0.0,arm.width/2.0,0.0),0,project.texture_metres);
        if let Some(surface)=&project.road_surface {
            let main=&project.arms[0];let d=direction(main.angle);let side=right(d);
            for v in &mut mesh.vertices[centre_start..] {
                let p=DVec2::new(v.position.x as f64,v.position.z as f64);
                v.uv=Vec2::new(((p.dot(side)+main.width/2.0)/surface.width_metres) as f32,
                    (p.dot(d)/project.texture_metres*if surface.reverse_v {-1.0} else {1.0}) as f32);
            }
        }
        let walk:Vec<_>=curve.iter().enumerate().map(|(j,&p)| {
            let tangent=if j==0 {-direction(arm.angle)} else if j+1==curve.len() {direction(next.angle)} else {(curve[j+1]-curve[j-1]).normalize_or_zero()};
            let outward=DVec2::new(-tangent.y,tangent.x);let width=arm.sidewalk+(next.sidewalk-arm.sidewalk)*j as f64/(curve.len()-1) as f64;
            (p.extend(0.15),(p+outward*width).extend(0.15))
        }).collect();ribbon(&mut mesh,&walk,1,project.texture_metres);
        if arm.sidewalk>0.0 || next.sidewalk>0.0 {
            let kerb:Vec<_>=curve.iter().map(|p|(p.extend(0.0),p.extend(0.15))).collect();ribbon(&mut mesh,&kerb,1,project.texture_metres);
        }
    }
    if omsi_geometry::outline_crosses_itself(&outline) {return Err("Straßenarme überschneiden sich; Biegung, Winkel oder Länge ändern".into());}
    for (i,a) in arms.iter().enumerate() {for (j,b) in arms.iter().enumerate() {if i==j {continue;}
        let start=point(a,core,0.0,-sign*a.width/4.0,0.0);let end=point(b,core,0.0,sign*b.width/4.0,0.0);
        let c1=start-direction(a.angle).extend(0.0)*(core*0.9);let c2=end-direction(b.angle).extend(0.0)*(core*0.9);
        let curve:Vec<_>=(0..=48).map(|k|cubic(start,c1,c2,end,k as f64/48.0)).collect();
        if curve.iter().any(|p|!contains(p.truncate(),&outline)) {return Err("Ein Fahrweg verlässt die Fahrbahn; Winkel oder Eckrundung anpassen".into());}
        let delta=(b.angle-a.angle-180.0+180.0).rem_euclid(360.0)-180.0;
        paths.push((curve,(a.width.min(b.width)/2.0).max(2.0),if delta.abs()<35.0 {1} else if delta>0.0 {3} else {2}));
    }}
    if mesh.vertices.len()>100_000 {return Err("Kreuzungsmodell ist zu groß".into());}
    Ok(Built {mesh,paths,outline})
}

fn path_overlay(mesh:&mut Mesh,paths:&[(Vec<DVec3>,f64,i32)]) {
    let slot=mesh.materials.len() as u16;mesh.materials.push(Material {diffuse:[0.08,0.6,1.0,1.0],..Default::default()});
    for (points,_,_) in paths {for pair in points.windows(2) {let d=(pair[1]-pair[0]).truncate().normalize_or_zero();let r=right(d)*0.055;
        quad(mesh,pair[0]+r.extend(0.03),pair[1]+r.extend(0.03),pair[1]-r.extend(-0.03),pair[0]-r.extend(-0.03),slot,1.0);
    }}
}

fn o3d(mesh:&Mesh)->Result<Vec<u8>,String> {
    let mut out=vec![0x84,0x19,4,1];out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.push(0x17);out.extend_from_slice(&(mesh.vertices.len() as u32).to_le_bytes());
    for v in &mesh.vertices {for f in [v.position.x,v.position.y,v.position.z,v.normal.x,v.normal.y,v.normal.z,v.uv.x,v.uv.y] {out.extend_from_slice(&f.to_le_bytes());}}
    out.push(0x49);out.extend_from_slice(&(mesh.triangles.len() as u32).to_le_bytes());
    for t in &mesh.triangles {for index in t.indices {out.extend_from_slice(&index.to_le_bytes());}out.extend_from_slice(&t.material.to_le_bytes());}
    out.push(0x26);out.extend_from_slice(&(mesh.materials.len() as u16).to_le_bytes());
    for m in &mesh.materials {for f in m.diffuse.into_iter().chain(m.specular).chain(m.emissive).chain([m.specular_power]) {out.extend_from_slice(&f.to_le_bytes());}
        if !m.texture.is_ascii() || m.texture.len()>255 {return Err("Exporttextur benötigt kurzen ASCII-Dateinamen".into());}
        out.push(m.texture.len() as u8);out.extend_from_slice(m.texture.as_bytes());}
    out.push(0x79);for f in Mat4::IDENTITY.to_cols_array() {out.extend_from_slice(&f.to_le_bytes());}Ok(out)
}

fn sco(project:&Project,built:&Built)->String {
    let mut text=format!("[friendlyname]\n{}\n\n[groups]\n2\nopenOMSI Editor\nEigene Kreuzungen\n\n[fixed]\n[surface]\n[absheight]\n\n[mesh]\nroad.o3d\n\n[terrainhole]\nroad.o3d\n",project.name);
    for (points,width,indicator) in &built.paths {for pair in points.windows(2) {
        let d=pair[1]-pair[0];let length=d.truncate().length();if length<0.011 {continue;}
        let heading=d.x.atan2(d.y).to_degrees().rem_euclid(360.0);
        text.push_str(&format!("\n[path]\n{:.6}\n{:.6}\n{:.6}\n{heading:.6}\n0\n{length:.6}\n0\n0\n0\n{width:.6}\n0\n{indicator}\n",pair[0].x,pair[0].y,pair[0].z));
    }}text
}

/// A unique asset directory for each export keeps previously placed crossings unchanged.
pub fn export(project:&Project,root:&Path,content:&Path,original:&Path)->Result<PathBuf,String> {
    let mut built=build(project)?;
    log::info!("junction builder: '{}' · {} outline points · {} driving paths",project.name,built.outline.len(),built.paths.len());
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let folder=content.join("Sceneryobjects/openOMSI_Editor/Junctions").join(format!("junction-{stamp}"));
    crate::editor::protect_output(&folder,original)?;
    let texture_dir=folder.join("texture");let model_dir=folder.join("model");
    // Resolve and read every required texture before creating the final asset.
    let mut textures=Vec::new();
    for (slot,base) in [(0,"asphalt"),(1,"gehweg")] {
        let name=&built.mesh.materials[slot].texture;if name.is_empty() {continue;}
        let path=omsi_texture::find_texture(name,&[root]).ok_or_else(||format!("Textur fehlt: {name}"))?;
        let decoded=omsi_texture::decode_file(&path).map_err(|e|e.to_string())?;
        if slot==0 {if let Some(surface)=&project.road_surface {
            let cropped=crop_surface(&decoded,surface)?;
            let image=image::RgbaImage::from_raw(cropped.width,cropped.height,cropped.rgba).ok_or("Ungültige Texturgröße")?;
            let mut bytes=std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(image).write_to(&mut bytes,image::ImageFormat::Png).map_err(|e|e.to_string())?;
            textures.push(("asphalt.png".into(),bytes.into_inner()));built.mesh.materials[slot].texture="asphalt.png".into();continue;
        }}
        let ext=path.extension().and_then(|e|e.to_str()).unwrap_or("dds").to_ascii_lowercase();
        let file=format!("{base}.{ext}");let bytes=omsi_cfg::vfs::read(&path).map_err(|e|e.to_string())?;
        let cfg=omsi_cfg::resolve_path(path.parent().unwrap_or(root),&format!("{}.cfg",path.file_name().unwrap_or_default().to_string_lossy()));
        if omsi_cfg::vfs::is_file(&cfg) {textures.push((format!("{file}.cfg"),omsi_cfg::vfs::read(&cfg).map_err(|e|e.to_string())?));}
        textures.push((file.clone(),bytes));built.mesh.materials[slot].texture=file;
    }
    let mesh_bytes=o3d(&built.mesh)?;let object_text=sco(project,&built);
    let json=serde_json::to_vec_pretty(project).map_err(|e|e.to_string())?;
    let result=(||->Result<PathBuf,String> {
        std::fs::create_dir_all(&texture_dir).map_err(|e|e.to_string())?;std::fs::create_dir_all(&model_dir).map_err(|e|e.to_string())?;
        for (file,bytes) in textures {crate::editor::save_copy(&texture_dir.join(file),&bytes)?;}
        crate::editor::save_copy(&model_dir.join("road.o3d"),&mesh_bytes)?;
        crate::editor::save_copy(&folder.join("junction.junction.json"),&json)?;
        let out=folder.join("junction.sco");crate::editor::save_copy(&out,&crate::editor::encode(&object_text,crate::editor::Encoding::Utf16Le))?;Ok(out)
    })();
    if result.is_err() {let _=std::fs::remove_dir_all(&folder);}result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn rotating_preview_keeps_pending_connection(){
        let mut w=Window::new(false);let pending=Some((((0,0),123),0));
        w.connection_preview(vec![[DVec3::ZERO,DVec3::X]],vec![],[None;4]);w.pending=pending;
        w.command(Command::Rotate);w.command(Command::ShowPaths);
        assert_eq!(w.pending,pending);assert_eq!(w.road_preview.len(),1);
        w.clear_connection_preview();assert!(w.pending.is_none());assert!(w.road_preview.is_empty());assert!(w.dirty);
    }

    #[test]
    fn t_and_x_meshes_roundtrip_and_paths_stay_on_the_road() {
        for cross in [false,true] {let mut p=Project::default();p.arms[3].enabled=cross;p.arms[0].width=11.0;p.arms[1].width=11.0;
            p.arms[2].width=7.0;p.arms[0].sidewalk=1.5;p.arms[1].sidewalk=1.5;
            let b=build(&p).unwrap();assert_eq!(b.paths.len(),if cross {20} else {12});
            assert!(b.paths.iter().all(|(points,_,_)|points.iter().all(|p|contains(p.truncate(),&b.outline))));
            let bytes=o3d(&b.mesh).unwrap();let restored=omsi_o3d::parse_o3d(&bytes).unwrap();
            assert_eq!(restored.vertices.len(),b.mesh.vertices.len());assert_eq!(restored.triangles,b.mesh.triangles);
            let object=omsi_scenery::SceneryObject::parse(&omsi_cfg::CfgFile::from_str("junction.sco",&sco(&p,&b)));
            assert!(object.absolute_height());assert!(object.paths.len()>50);assert!(object.paths.iter().all(|p|p.kind==0&&p.direction==0));
            assert!(!omsi_geometry::outline_crosses_itself(&b.outline));
        }
    }
    #[test]
    fn independent_widths_curvature_undo_and_bad_geometry_are_checked() {
        let mut w=Window::new(false);w.arm=0;w.set(Field::Width,11.0);w.set(Field::Bend,-15.0);
        assert_eq!(w.project.arms[1].width,7.0);assert!(build(&w.project).is_ok());
        w.command(Command::Undo);assert_eq!(w.project.arms[0].bend,0.0);w.command(Command::Redo);assert_eq!(w.project.arms[0].bend,-15.0);
        w.project.arms[2].angle=90.0;assert!(build(&w.project).is_err());
        w.project=Project::default();w.project.road_texture="../bad.dds".into();assert!(build(&w.project).is_err());
        w.project=Project::default();w.project.arms[0].width=f64::NAN;assert!(build(&w.project).is_err());
    }
    #[test]
    fn exported_asset_loads_in_the_existing_object_pipeline_and_project_reopens() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("omsi-junction-export-{stamp}"));let original=dir.join("original");
        let content=dir.join("content");std::fs::create_dir_all(&original).unwrap();let project=Project::default();
        let sco=export(&project,&original,&content,&original).unwrap();omsi_cfg::content_changed();
        let (object,_,model,model_dir)=crate::scene::scenery_definition(&original,&sco.to_string_lossy()).unwrap();
        assert!(object.surface&&object.absolute_height());assert!(object.paths.len()>50);assert_eq!(model.meshes.len(),1);
        let mesh_path=crate::scene::scenery_mesh_path(&model_dir,&model.meshes[0].file);
        let mesh=omsi_o3d::load_mesh(&mesh_path).unwrap();assert!(!mesh.vertices.is_empty());
        let mut window=Window::new(false);window.load(&sco.parent().unwrap().join("junction.junction.json")).unwrap();
        assert_eq!(window.project,project);assert!(crate::asset_catalog::junction_preview(&original,&mesh,0,None).is_ok());
        assert!(!original.join("Sceneryobjects").exists());let again=export(&project,&original,&content,&original).unwrap();assert_ne!(sco,again);
        let _=std::fs::remove_dir_all(dir);
    }
    #[test]
    fn atlas_crop_excludes_adjacent_stripes_and_supports_mirror() {
        let image=omsi_texture::Image {width:4,height:1,rgba:vec![255,0,0,255, 30,30,30,255, 60,60,60,255, 255,255,255,255],has_alpha:false};
        let mut surface=RoadSurface {u_start:0.25,u_end:0.75,reverse_v:false,width_metres:7.0};
        let crop=crop_surface(&image,&surface).unwrap();assert_eq!(crop.width,2);assert_eq!(crop.rgba,vec![30,30,30,255,60,60,60,255]);
        surface.u_start=0.75;surface.u_end=0.25;
        assert_eq!(crop_surface(&image,&surface).unwrap().rgba,vec![60,60,60,255,30,30,30,255]);
        surface.u_end=surface.u_start;assert!(crop_surface(&image,&surface).is_err());
    }
    #[test]
    fn road_surface_comes_from_driving_profile_not_first_texture() {
        let def=omsi_scenery::Spline::parse(&omsi_cfg::CfgFile::from_str("road.sli",
            "[texture]\nkerb.bmp\n[texture]\nasphalt.bmp\n[profile]\n0\n[profilepnt]\n5\n0\n0\n0.25\n[profilepnt]\n6\n0\n1\n0.25\n[profile]\n1\n[profilepnt]\n-3.5\n0\n0.25\n0.2\n[profilepnt]\n3.5\n0\n0.75\n0.2\n[path]\n0\n0\n0\n7\n0\n"));
        let (slot,surface,metres)=spline_surface(&def,false,20.0).unwrap();assert_eq!(slot,1);assert!((metres-5.0).abs()<1e-5);
        assert_eq!((surface.u_start,surface.u_end),(0.25,0.75));
        let (_,mirrored,_)=spline_surface(&def,true,20.0).unwrap();assert_eq!((mirrored.u_start,mirrored.u_end),(0.75,0.25));assert!(mirrored.reverse_v);
        let mut missing=def.clone();missing.paths.clear();assert!(spline_surface(&missing,false,20.0).is_err());
    }
    #[test]
    fn old_projects_keep_uvs_new_projects_follow_curved_arms_and_roundtrip() {
        let mut json=serde_json::to_value(Project::default()).unwrap();json.as_object_mut().unwrap().remove("road_surface");
        let mut old:Project=serde_json::from_value(json).unwrap();assert!(old.road_surface.is_none());old.markings=false;
        let old_mesh=build(&old).unwrap().mesh;
        assert!(old_mesh.vertices.iter().all(|v|(v.uv.x-v.position.x/old.texture_metres as f32).abs()<1e-5));
        let mut project=Project::default();project.arms[1].bend=20.0;
        let restored:Project=serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();assert_eq!(project,restored);
        let built=build(&project).unwrap();
        // First sorted arm is 90 degrees, bent: lateral UVs retain the physical texture repeat width.
        let core=project.arms.iter().filter(|a|a.enabled).map(|a|a.width/2.0+a.sidewalk).fold(0.0,f64::max)+project.corner+1.0;
        let count=((project.arms[1].length-core)/0.75).ceil() as usize*6;
        assert!(built.mesh.vertices[..count].iter().all(|v|v.uv.x==0.0||v.uv.x==(project.arms[1].width/project.road_surface.as_ref().unwrap().width_metres) as f32));
        assert!(built.mesh.vertices[..count].iter().all(|v|v.uv.is_finite()));
    }
    #[test]
    fn surface_edit_and_spline_import_are_undoable() {
        let mut w=Window::new(false);let original=w.project.clone();
        w.use_spline(8.0,"Texture/asphalt.bmp".into(),RoadSurface {u_start:0.2,u_end:0.4,reverse_v:true,width_metres:4.0},5.0);
        let imported=w.project.clone();w.command(Command::Undo);assert_eq!(w.project,original);
        w.command(Command::Redo);assert_eq!(w.project,imported);
        w.set(Field::UStart,0.3);w.command(Command::Undo);assert_eq!(w.project,imported);
    }

    #[test]
    fn cropped_export_contains_only_selected_pixels_and_reloads_uv_settings() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("omsi-surface-export-{stamp}"));
        let original=dir.join("original");let content=dir.join("content");
        std::fs::create_dir_all(original.join("Texture")).unwrap();
        let pixels=vec![255,0,0,255, 30,30,30,255, 60,60,60,255, 255,255,255,255];
        image::RgbaImage::from_raw(4,1,pixels).unwrap().save(original.join("Texture/atlas.png")).unwrap();
        let mut project=Project::default();project.road_texture="Texture/atlas.png".into();
        project.road_surface=Some(RoadSurface {u_start:0.25,u_end:0.75,reverse_v:true,width_metres:4.0});
        let sco=export(&project,&original,&content,&original).unwrap();
        let folder=sco.parent().unwrap();let image=omsi_texture::decode_file(&folder.join("texture/asphalt.png")).unwrap();
        assert_eq!(image.rgba,vec![30,30,30,255,60,60,60,255]);
        let mesh=omsi_o3d::load_mesh(&folder.join("model/road.o3d")).unwrap();assert_eq!(mesh.materials[0].texture,"asphalt.png");
        let mut window=Window::new(false);window.load(&folder.join("junction.junction.json")).unwrap();assert_eq!(window.project,project);
        let expected=build(&project).unwrap().mesh;assert_eq!(mesh.vertices.len(),expected.vertices.len());
        for (a,b) in mesh.vertices.iter().zip(&expected.vertices) {assert_eq!(a.uv,b.uv);assert_eq!(a.position,b.position);}
        std::fs::remove_dir_all(dir).unwrap();
    }

}

#[cfg(test)]
mod texture_step_tests {
    use super::*;
    fn type_value(w:&mut Window,f:Field,text:&str) {
        w.edit(f);let input=w.input.as_mut().unwrap();input.text=text.into();input.replace=false;
    }
    #[test]
    fn typed_precision_and_clicked_place_control_all_three_fields() {
        for f in [Field::UStart,Field::UEnd,Field::TextureWidth] {
            let mut w=Window::new(false);
            type_value(&mut w,f,"6,5");w.command(Command::Step(f,1.0));assert!((w.value(f)-6.6).abs()<1e-9);
            type_value(&mut w,f,"6.52");w.command(Command::Step(f,1.0));assert!((w.value(f)-6.53).abs()<1e-9);
            w.command(Command::Digit(f,0));w.command(Command::Step(f,1.0));assert!((w.value(f)-7.53).abs()<1e-9);
            w.command(Command::Step(f,-1.0));assert!((w.value(f)-6.53).abs()<1e-9);
            w.command(Command::Digit(f,-1));w.command(Command::Step(f,-1.0));assert!((w.value(f)-6.43).abs()<1e-9);
        }
    }
    #[test]
    fn bounds_invalid_text_and_undo_keep_project_safe() {
        let mut w=Window::new(false);
        type_value(&mut w,Field::TextureWidth,"1");w.command(Command::Step(Field::TextureWidth,-1.0));
        assert_eq!(w.value(Field::TextureWidth),0.25);assert_eq!(w.number_text(Field::TextureWidth),"0.25");
        w.command(Command::Undo);assert_eq!(w.value(Field::TextureWidth),1.0);
        w.command(Command::Redo);assert_eq!(w.value(Field::TextureWidth),0.25);
        type_value(&mut w,Field::UStart,"-16");w.command(Command::Step(Field::UStart,-1.0));assert_eq!(w.value(Field::UStart),-16.0);
        type_value(&mut w,Field::UEnd,"16");w.command(Command::Step(Field::UEnd,1.0));assert_eq!(w.value(Field::UEnd),16.0);
        let before=w.project.clone();
        for bad in ["-", "NaN", "6.12345", "17"] {
            type_value(&mut w,Field::UStart,bad);w.command(Command::Step(Field::UStart,1.0));assert_eq!(w.project,before);assert!(w.input.is_some());
        }
    }
    #[test]
    fn undo_does_not_hide_or_round_restored_decimal_places() {
        let mut w=Window::new(false);let f=Field::TextureWidth;
        type_value(&mut w,f,"6.52");assert!(w.commit());
        type_value(&mut w,f,"6.5");assert!(w.commit());
        w.command(Command::Undo);assert_eq!(w.number_text(f),"6.52");
        w.command(Command::Step(f,1.0));assert!((w.value(f)-6.62).abs()<1e-9);
    }
    #[test]
    fn repeated_steps_preserve_precision_and_field_independence() {
        let mut w=Window::new(false);
        type_value(&mut w,Field::UStart,"0.00");assert!(w.commit());
        type_value(&mut w,Field::TextureWidth,"6.5");assert!(w.commit());
        for _ in 0..100 {w.command(Command::Step(Field::UStart,1.0));}
        assert_eq!(w.number_text(Field::UStart),"1.00");assert_eq!(w.texture_step(Field::TextureWidth),0.1);
        type_value(&mut w,Field::UStart,"-0.01");w.command(Command::Step(Field::UStart,1.0));assert_eq!(w.number_text(Field::UStart),"0.00");
    }
}
