//! Terrain painting and whole-tile height changes, grouped by gesture for undo/redo.

use crate::scene::World;
use glam::DVec3;
use hashbrown::{HashMap, HashSet};
use omsi_map::Terrain;

pub type Key = (i32, i32);
pub const HEIGHT_MIN: f64 = -1000.0;
pub const HEIGHT_MAX: f64 = 10000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Heights, Textures }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool { Raise, Lower, Level, Smooth, Height }
impl Tool {
    pub fn title(self) -> &'static str {
        match self { Self::Raise => "Berg heben", Self::Lower => "Senke senken", Self::Level => "Begradigen",
            Self::Smooth => "Glätten", Self::Height => "Höhe zeichnen" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field { Radius, Strength, Softness, Height, TileStep }
impl Field {
    pub fn title(self) -> &'static str {
        match self { Self::Radius => "Radius (m)", Self::Strength => "Stärke / Sek.", Self::Softness => "Weicher Rand (%)",
            Self::Height => "Zielhöhe (m)", Self::TileStep => "Tile-Schritt (m)" }
    }
    fn limits(self) -> (f64, f64) {
        match self { Self::Radius => (2.5,150.0), Self::Strength => (0.05,10.0), Self::Softness => (0.0,100.0),
            Self::Height => (HEIGHT_MIN,HEIGHT_MAX), Self::TileStep => (0.05,100.0) }
    }
}

#[derive(Clone, Copy)]
pub enum Command { Exit, Tool(Tool), FineSmooth, Edit(Field), Adjust(Field,f64), SampleHeight, PickTile,
    TileMove(f64), TileLevel, Blend, Undo, Redo, Save, Mode(Mode), TextureCatalog, Layer(i32), Erase(bool), SampleTexture }

pub struct NumberInput { pub field: Field, pub text: String, pub replace: bool }

struct Change { before: HashMap<Key,Terrain>, after: HashMap<Key,Terrain>,
    before_masks:HashMap<crate::ground_paint::Key,crate::ground_paint::Mask>,
    after_masks:HashMap<crate::ground_paint::Key,crate::ground_paint::Mask> }
impl Change {
    fn bytes(&self) -> usize { self.before.values().chain(self.after.values()).map(|t|t.heights.len()*4).sum::<usize>()
        +self.before_masks.values().chain(self.after_masks.values()).map(|m|m.alpha.len()*4).sum::<usize>() }
}
struct Stroke { before: HashMap<Key,Terrain>, before_masks:HashMap<crate::ground_paint::Key,crate::ground_paint::Mask>,
    paint_tiles:HashSet<Key>,last: DVec3, level: f64, error: Option<String> }

pub struct TerrainEditor {
    pub active: bool,
    pub mode: Mode,
    pub texture_layer: usize,
    pub texture_name: String,
    pub texture_erase: bool,
    pub sample_texture: bool,
    pub tool: Tool,
    pub radius: f64,
    pub strength: f64,
    pub softness: f64,
    pub height: f64,
    pub tile_step: f64,
    pub blend: bool,
    pub tile: Option<Key>,
    pub cursor: Option<DVec3>,
    pub sample_height: bool,
    pub pick_tile: bool,
    pub input: Option<NumberInput>,
    pub message: String,
    stroke: Option<Stroke>,
    undo: Vec<Change>,
    redo: Vec<Change>,
    pending: HashSet<Key>,
    refresh: f32,
}

impl Default for TerrainEditor {
    fn default() -> Self { Self { active:false, mode:Mode::Heights,texture_layer:0,texture_name:"Grundtextur".into(),
        texture_erase:false,sample_texture:false,tool:Tool::Raise, radius:10.0, strength:1.0, softness:50.0,
        height:0.0, tile_step:1.0, blend:true, tile:None, cursor:None, sample_height:false, pick_tile:false,
        input:None, message:"Links halten und zeichnen · Mausrad: Pinselgröße · Strg+S: speichern".into(),
        stroke:None, undo:Vec::new(), redo:Vec::new(), pending:HashSet::new(), refresh:0.0 } }
}

pub fn parse_number(text: &str, min: f64, max: f64) -> Result<f64,String> {
    text.trim().replace(',',".").parse::<f64>().ok().filter(|n|n.is_finite() && *n>=min && *n<=max)
        .ok_or_else(|| format!("Zahl zwischen {min} und {max} eingeben"))
}

fn tile_at(p: DVec3) -> Key {
    let s=omsi_map::tile_size(); ((p.x/s).floor() as i32,(p.y/s).floor() as i32)
}

fn read_terrain(world: &World, key: Key) -> Result<Terrain,String> {
    let src=world.tile_source(key.0,key.1).ok_or("Tile liegt außerhalb der Karte")?;
    // This also applies to a whole-tile change with neighbour blending disabled.
    if !world.chrono_dirs.read().is_empty()
        && crate::tiles::read_tile(&src,&world.chrono_dirs.read()).is_some_and(|t|t.terrain_from.is_some()) {
        return Err("Gelände mit aktiver Chrono-Änderung ist hier nicht bearbeitbar".into());
    }
    if let Some(t)=world.editor_terrain_tile(key) { return valid(t,key); }
    if !omsi_cfg::vfs::is_file(&src) { return Err("Tile-Datei fehlt".into()); }
    let path=crate::scene::tile_companion(&src,".terrain");
    let terrain=if omsi_cfg::vfs::is_file(&path) { Terrain::load(&path).map_err(|e|e.to_string())? } else { Terrain::flat() };
    valid(terrain,key)
}
fn valid(t: Terrain,key: Key) -> Result<Terrain,String> {
    if t.cells==0 || t.cells>256 || t.heights.len()!=t.samples()*t.samples() || t.heights.iter().any(|h|!h.is_finite()) {
        return Err(format!("Ungültiges Geländeraster in Tile ({},{})",key.0,key.1));
    }
    Ok(t)
}

fn distance_to_tile(x:f64,y:f64,key:Key) -> f64 {
    let s=omsi_map::tile_size(); let (a,b)=(key.0 as f64*s,key.1 as f64*s);
    let dx=(a-x).max(0.0).max(x-a-s); let dy=(b-y).max(0.0).max(y-b-s);
    dx.hypot(dy)
}

fn footprint(world:&World,at:DVec3,radius:f64) -> Result<HashMap<Key,Terrain>,String> {
    let s=omsi_map::tile_size(); let mut out=HashMap::new();
    // Include shared vertices at exact borders, even if the radius is zero there.
    for x in ((at.x-radius)/s).floor() as i32-1..=((at.x+radius)/s).floor() as i32 {
        for y in ((at.y-radius)/s).floor() as i32-1..=((at.y+radius)/s).floor() as i32 {
            let k=(x,y);
            if distance_to_tile(at.x,at.y,k)>radius+1e-6 || world.tile_source(x,y).is_none() { continue; }
            out.insert(k,read_terrain(world,k)?);
        }
    }
    if let Some(cells)=out.values().next().map(|t|t.cells) {
        if out.values().any(|t|t.cells!=cells) {return Err("Angrenzende Tiles haben unterschiedliche Geländeraster".into());}
    }
    Ok(out)
}

/// Exact edge samples matter when smoothing duplicated vertices on adjacent tiles.
fn sample(tiles:&HashMap<Key,Terrain>,x:f64,y:f64) -> Option<f64> {
    let s=omsi_map::tile_size(); let k=tile_at(DVec3::new(x,y,0.0));
    for candidate in [k,(k.0-1,k.1),(k.0,k.1-1),(k.0-1,k.1-1)] {
        let Some(t)=tiles.get(&candidate) else { continue; };
        let lx=x-candidate.0 as f64*s; let ly=y-candidate.1 as f64*s;
        if lx < -1e-6 || ly < -1e-6 || lx > s+1e-6 || ly > s+1e-6 { continue; }
        let (fx,fy)=(lx/s*t.cells as f64,ly/s*t.cells as f64);
        if (fx-fx.round()).abs()<1e-6 && (fy-fy.round()).abs()<1e-6 {
            return Some(t.height_at(fx.round().clamp(0.0,t.cells as f64) as usize,fy.round().clamp(0.0,t.cells as f64) as usize) as f64);
        }
        return Some(t.sample(lx as f32,ly as f32) as f64);
    }
    None
}

pub(crate) fn falloff(distance:f64,radius:f64,softness:f64) -> f64 {
    let core=radius*(1.0-softness/100.0);
    if distance>=radius { 0.0 } else if distance<=core { 1.0 }
    else { let t=(distance-core)/(radius-core); 1.0-t*t*(3.0-2.0*t) }
}

fn paint(before:&HashMap<Key,Terrain>,at:DVec3,radius:f64,softness:f64,tool:Tool,target:f64,amount:f64) -> HashMap<Key,Terrain> {
    let s=omsi_map::tile_size(); let mut after=HashMap::new();
    for (&key,old) in before {
        let mut t=old.clone(); let n=t.samples(); let step=s/t.cells as f64; let mut changed=false;
        for iy in 0..n { for ix in 0..n {
            let (x,y)=(key.0 as f64*s+ix as f64*step,key.1 as f64*s+iy as f64*step);
            let w=falloff((x-at.x).hypot(y-at.y),radius,softness); if w<=0.0 { continue; }
            let index=iy*n+ix; let h=sample(before,x,y).unwrap_or(t.heights[index] as f64);
            let new=match tool {
                Tool::Raise => h+amount*w, Tool::Lower => h-amount*w,
                Tool::Level | Tool::Height => h+(target-h)*w,
                Tool::Smooth => {
                    let mut sum=0.0; let mut count=0.0;
                    for dy in -1..=1 { for dx in -1..=1 {
                        if let Some(v)=sample(before,x+dx as f64*step,y+dy as f64*step) { sum+=v; count+=1.0; }
                    } }
                    h+(sum/count-h)*(1.0-(-amount*4.0).exp())*w
                }
            };
            let value=new.clamp(HEIGHT_MIN,HEIGHT_MAX) as f32;
            if (value-t.heights[index]).abs()>1e-5 { t.heights[index]=value; changed=true; }
        } }
        if changed { after.insert(key,t); }
    }
    after
}

/// Include a smooth band in every neighbour, including diagonal corner neighbours.
fn tile_height(before:&HashMap<Key,Terrain>,tile:Key,value:f64,level:bool,blend:bool) -> HashMap<Key,Terrain> {
    let s=omsi_map::tile_size(); let mut after=HashMap::new();
    let target: HashMap<_,_> = before.get(&tile).map(|t|(tile,t.clone())).into_iter().collect();
    for (&key,old) in before {
        if key!=tile && !blend { continue; }
        let mut t=old.clone(); let n=t.samples(); let step=s/t.cells as f64;
        for y in 0..n { for x in 0..n {
            let (wx,wy)=(key.0 as f64*s+x as f64*step,key.1 as f64*s+y as f64*step);
            let d=distance_to_tile(wx,wy,tile); if key!=tile && d>=20.0 { continue; }
            let w=if d<=0.0 { 1.0 } else { let f=d/20.0; 1.0-f*f*(3.0-2.0*f) };
            let i=y*n+x;
            // Both copies of the target tile's border use that tile's height.
            let h=if d<=1e-6 {
                sample(&target,wx,wy).unwrap_or(t.heights[i] as f64)
            } else { t.heights[i] as f64 };
            t.heights[i]=(if level { h+(value-h)*w } else { h+value*w }).clamp(HEIGHT_MIN,HEIGHT_MAX) as f32;
        } }
        if t!=*old { after.insert(key,t); }
    }
    after
}

impl TerrainEditor {
    pub fn describe(&self) -> String {
        if self.mode==Mode::Textures {return format!("Bodentexturen · {} · {} · Radius {:.1} m · {} Rückgängig / {} Wiederholen",
            self.texture_name,if self.texture_erase {"Radierer"} else {"Malen"},self.radius,self.undo.len(),self.redo.len());}
        format!("Gelände · {} · Radius {:.1} m · Zielhöhe {:.2} m · {} Rückgängig / {} Wiederholen",
            self.tool.title(),self.radius,self.height,self.undo.len(),self.redo.len())
    }
    pub fn can_undo(&self)->bool { !self.undo.is_empty() }
    pub fn can_redo(&self)->bool { !self.redo.is_empty() }
    pub fn painting(&self)->bool { self.stroke.is_some() }
    pub fn value(&self,field:Field)->f64 {
        match field { Field::Radius=>self.radius, Field::Strength=>self.strength, Field::Softness=>self.softness,
            Field::Height=>self.height, Field::TileStep=>self.tile_step }
    }
    pub fn set_value(&mut self,field:Field,value:f64) {
        let (lo,hi)=field.limits(); let value=value.clamp(lo,hi);
        match field { Field::Radius=>self.radius=value, Field::Strength=>self.strength=value, Field::Softness=>self.softness=value,
            Field::Height=>self.height=value, Field::TileStep=>self.tile_step=value }
    }
    pub fn edit(&mut self,field:Field) {
        self.input=Some(NumberInput { field,text:format!("{:.2}",self.value(field)),replace:true });
    }
    pub fn commit_input(&mut self)->bool {
        let Some(input)=self.input.as_ref() else { return true; };
        let field=input.field; let (lo,hi)=field.limits();
        match parse_number(&input.text,lo,hi) { Ok(n)=>{ self.set_value(field,n); self.input=None; true },
            Err(e)=>{ self.message=e; false } }
    }
    pub fn begin(&mut self,world:&World,at:DVec3) {
        if self.mode==Mode::Textures && self.sample_texture {
            match crate::ground_paint::sample(world,at) {Ok(layer)=>{self.choose_layer(world,layer);
                self.message=format!("Textur aufgenommen: {}",self.texture_name);},Err(e)=>self.message=e}
            self.sample_texture=false;return;
        }
        if self.sample_height { self.height=at.z; self.sample_height=false; self.message=format!("Zielhöhe {:.2} m aufgenommen",at.z); return; }
        self.tile=Some(tile_at(at));
        if self.pick_tile { self.pick_tile=false; self.message="Tile ausgewählt; Höhenbuttons bearbeiten das ganze Feld".into(); return; }
        self.stroke=Some(Stroke { before:HashMap::new(),before_masks:HashMap::new(),paint_tiles:HashSet::new(),last:at,level:at.z,error:None });
        self.dab(world,at,1.0/30.0);
    }
    pub fn once(&mut self,world:&World,at:DVec3,tool:Tool,amount:f64) -> Vec<Key> {
        self.finish(world);
        let before=match footprint(world,at,self.radius+10.0) { Ok(t)=>t,Err(e)=>{self.message=e;return Vec::new();} };
        let after=paint(&before,at,self.radius,self.softness,tool,at.z,amount);
        let keys:Vec<_>=after.keys().copied().collect();
        if keys.is_empty() {self.message="Keine Höhenänderung; gegebenenfalls den Pinselradius erhöhen".into();}
        { let mut edits=world.terrain_edits.lock();for (&k,t) in &after {edits.insert(k,t.clone());self.pending.insert(k);} }
        self.remember(before,after); keys
    }
    fn dab(&mut self,world:&World,at:DVec3,dt:f64) {
        if self.mode==Mode::Textures {self.dab_texture(world,at,dt);return;}
        let Some(stroke)=self.stroke.as_ref() else { return; };
        let level=if self.tool==Tool::Height { self.height } else { stroke.level };
        let before=match footprint(world,at,self.radius+10.0) { Ok(t)=>t,Err(e)=>{
            self.stroke.as_mut().unwrap().error=Some(e.clone());self.message=e;return;
        } };
        let after=paint(&before,at,self.radius,self.softness,self.tool,level,self.strength*dt);
        let stroke=self.stroke.as_mut().unwrap();
        let mut edits=world.terrain_edits.lock();
        for (key,t) in after {
            stroke.before.entry(key).or_insert_with(||before[&key].clone());
            edits.insert(key,t); self.pending.insert(key);
        }
    }
    pub fn move_brush(&mut self,world:&World,at:DVec3,dt:f32) {
        let Some(stroke)=self.stroke.as_ref() else { return; };
        let last=stroke.last; let distance=(at-last).truncate().length();
        let count=(distance/(self.radius*0.25).max(1.0)).ceil().clamp(1.0,64.0) as usize;
        for i in 1..=count { self.dab(world,last.lerp(at,i as f64/count as f64),dt as f64/count as f64); }
        if let Some(stroke)=self.stroke.as_mut() { stroke.last=at; }
    }
    fn remember(&mut self,before:HashMap<Key,Terrain>,after:HashMap<Key,Terrain>) {
        if after.is_empty() { return; }
        let before=before.into_iter().filter(|(k,_)|after.contains_key(k)).collect();
        self.undo.push(Change { before,after,before_masks:HashMap::new(),after_masks:HashMap::new() }); self.redo.clear();
        while self.undo.len()>128 || (self.undo.len()>1 && self.undo.iter().map(Change::bytes).sum::<usize>()>64*1024*1024) {
            self.undo.remove(0);
        }
    }
    pub fn finish(&mut self,world:&World) {
        let Some(stroke)=self.stroke.take() else { return; };
        let after={ let edits=world.terrain_edits.lock(); stroke.before.iter().filter_map(|(k,t)|
            edits.get(k).filter(|now|*now!=t).map(|now|(*k,now.clone()))).collect::<HashMap<_,_>>() };
        let after_masks={let edits=world.ground_paint_edits.lock();stroke.before_masks.iter().filter_map(|(k,m)|
            edits.get(k).filter(|now|*now!=m).map(|now|(*k,now.clone()))).collect::<HashMap<_,_>>()};
        let count=after.len()+after_masks.keys().map(|k|k.0).collect::<HashSet<_>>().len();
        if !after_masks.is_empty() {
            self.undo.push(Change {before:HashMap::new(),after:HashMap::new(),before_masks:stroke.before_masks,after_masks});
            self.redo.clear();
            while self.undo.len()>128 || (self.undo.len()>1 && self.undo.iter().map(Change::bytes).sum::<usize>()>64*1024*1024) {self.undo.remove(0);}
        }
        self.remember(stroke.before,after);
        if count>0 { self.message=format!("Pinselstrich: {count} Tile(s) · Strg+Z rückgängig · Strg+S speichern"); }
        else {self.message=if self.mode==Mode::Textures {"Keine Texturänderung"} else {"Keine Höhenänderung; bei kleinem Pinsel gegebenenfalls den Radius erhöhen"}.into();}
        if let Some(error)=stroke.error {self.message=if count>0 {format!("{} · {error}",self.message)} else {error};}
        if self.mode==Mode::Textures {log::info!("Texturpinsel · Ebene {} '{}' · {}",self.texture_layer,self.texture_name,self.message);}
    }
    pub fn change_tile(&mut self,world:&World,value:f64,level:bool) {
        self.finish(world);
        let Some(key)=self.tile else { self.message="Zuerst Tile auswählen".into(); return; };
        let s=omsi_map::tile_size(); let at=DVec3::new((key.0 as f64+0.5)*s,(key.1 as f64+0.5)*s,0.0);
        let before=if self.blend {
            match footprint(world,at,s/std::f64::consts::SQRT_2+21.0) { Ok(t)=>t,Err(e)=>{self.message=e;return;} }
        } else {match read_terrain(world,key) {Ok(t)=>[(key,t)].into_iter().collect(),Err(e)=>{self.message=e;return;}}};
        if !before.contains_key(&key) { self.message="Gewähltes Tile ist nicht verfügbar".into();return; }
        let after=tile_height(&before,key,value,level,self.blend);
        { let mut edits=world.terrain_edits.lock(); for (&k,t) in &after { edits.insert(k,t.clone());self.pending.insert(k); } }
        self.remember(before,after);
        self.message=if level { format!("Tile ({},{}) auf {:.2} m gesetzt",key.0,key.1,value) }
            else { format!("Tile ({},{}) um {value:+.2} m verschoben",key.0,key.1) };
    }
    pub fn undo_redo(&mut self,world:&World,redo:bool) {
        self.finish(world);
        let stack=if redo { &self.redo } else { &self.undo };
        let Some(change)=stack.last() else { self.message="Keine Geländeänderung verfügbar".into(); return; };
        let expected=if redo { &change.before } else { &change.after };
        for (key,t) in expected {
            if read_terrain(world,*key).as_ref()!=Ok(t) {
                self.message="Neuere Geländeänderung aus anderem Werkzeug: diese zuerst dort rückgängig machen".into();return;
            }
        }
        let expected_masks=if redo {&change.before_masks} else {&change.after_masks};
        for (key,mask) in expected_masks {
            if crate::ground_paint::read(world,*key).as_ref()!=Ok(mask) {
                self.message="Bodentexturen wurden zwischenzeitlich geändert; nichts rückgängig gemacht".into();return;
            }
        }
        let change=if redo { self.redo.pop().unwrap() } else { self.undo.pop().unwrap() };
        let restore=if redo { &change.after } else { &change.before };
        { let mut edits=world.terrain_edits.lock();for (&key,t) in restore { edits.insert(key,t.clone());self.pending.insert(key); } }
        let masks=if redo {&change.after_masks} else {&change.before_masks};
        {let mut edits=world.ground_paint_edits.lock();for (&key,mask) in masks {edits.insert(key,mask.clone());self.pending.insert(key.0);}}
        if redo { self.undo.push(change); } else { self.redo.push(change); }
        self.message=if redo { "Geländeänderung wiederholt" } else { "Geländeänderung rückgängig" }.into();
    }
    pub fn choose_layer(&mut self,world:&World,layer:usize) {
        let defs=crate::ground_paint::layers(world);if let Some(def)=defs.get(layer) {
            self.texture_layer=layer;self.texture_name=def.texture.clone();self.sample_texture=false;
            if layer==0 {self.texture_erase=false;}
        }
    }
    fn dab_texture(&mut self,world:&World,at:DVec3,dt:f64) {
        let Some(stroke)=self.stroke.as_ref() else {return;};if stroke.error.is_some() {return;}
        let before=match crate::ground_paint::footprint(world,at,self.radius,self.texture_layer,self.texture_erase,&mut self.stroke.as_mut().unwrap().paint_tiles) {
            Ok(masks)=>masks,Err(e)=>{self.stroke.as_mut().unwrap().error=Some(e.clone());self.message=e;return;}
        };
        let after=crate::ground_paint::dab(&before,at,self.radius,self.softness,self.texture_layer,self.texture_erase,self.strength*dt);
        let stroke=self.stroke.as_mut().unwrap();let mut edits=world.ground_paint_edits.lock();
        for (key,mask) in after {stroke.before_masks.entry(key).or_insert_with(||before[&key].clone());
            edits.insert(key,mask);self.pending.insert(key.0);}
    }
    pub fn take_pending(&mut self,dt:f32,force:bool)->Vec<Key> {
        self.refresh+=dt;
        if !force && self.refresh<0.25 { return Vec::new(); }
        self.refresh=0.0; self.pending.drain().collect()
    }
    pub fn ground_hit(world:&World,origin:DVec3,direction:DVec3)->Option<DVec3> {
        let above=|t:f64| { let p=origin+direction*t;world.editor_terrain_height(p.x,p.y).map(|h|p.z>h) };
        let mut t=0.5;let mut last=0.0;
        while t<800.0 {
            if above(t)==Some(false) {
                let (mut a,mut b)=(last,t);
                for _ in 0..24 { let m=(a+b)*0.5;if above(m).unwrap_or(true) {a=m;} else {b=m;} }
                let p=origin+direction*b;
                return world.editor_terrain_height(p.x,p.y).map(|h|DVec3::new(p.x,p.y,h));
            }
            last=t;t+=(t*0.01).max(0.25);
        }
        None
    }
    pub fn markers(&self,world:&World)->Vec<DVec3> {
        let Some(at)=self.cursor else { return Vec::new(); };
        let mut out=Vec::new();
        for i in 0..64 {
            let angle=i as f64*std::f64::consts::TAU/64.0;
            let (x,y)=(at.x+angle.cos()*self.radius,at.y+angle.sin()*self.radius);
            if let Some(h)=world.editor_terrain_height(x,y) { out.push(DVec3::new(x,y,h+0.15)); }
        }
        out.push(at+DVec3::Z*0.2);out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flat(height:f32)->Terrain { let mut t=Terrain::flat();t.heights.fill(height);t }
    #[test]
    fn raise_and_lower_are_symmetric_and_leave_outer_points_unchanged() {
        let before=[((0,0),flat(40.0))].into_iter().collect();
        let at=DVec3::new(150.0,150.0,40.0);
        let raised=paint(&before,at,20.0,50.0,Tool::Raise,0.0,2.0);
        assert_eq!(raised[&(0,0)].height_at(30,30),42.0);
        assert_eq!(raised[&(0,0)].height_at(0,0),40.0);
        let lowered=paint(&raised,at,20.0,50.0,Tool::Lower,0.0,2.0);
        for (a,b) in lowered[&(0,0)].heights.iter().zip(before[&(0,0)].heights.iter()) { assert!((a-b).abs()<0.00001); }
    }
    #[test]
    fn fixed_height_is_absolute_and_smoothing_removes_a_peak_without_overshoot() {
        let mut t=flat(40.0);t.heights[30*61+30]=50.0;
        let before=[((0,0),t)].into_iter().collect();let at=DVec3::new(150.0,150.0,50.0);
        let height=paint(&before,at,20.0,50.0,Tool::Height,45.5,0.1);
        assert_eq!(height[&(0,0)].height_at(30,30),45.5);
        let smooth=paint(&before,at,20.0,50.0,Tool::Smooth,0.0,0.2);
        assert!(smooth[&(0,0)].height_at(30,30)<50.0);
        assert!(smooth[&(0,0)].heights.iter().all(|h|*h>=40.0 && *h<=50.0));
    }
    #[test]
    fn brush_and_tile_height_keep_shared_edges_equal_and_fade_into_neighbours() {
        let before=[((0,0),flat(40.0)),((1,0),flat(40.0))].into_iter().collect();
        let p=paint(&before,DVec3::new(300.0,150.0,40.0),20.0,50.0,Tool::Raise,0.0,2.0);
        for j in 0..61 { assert_eq!(p[&(0,0)].height_at(60,j),p[&(1,0)].height_at(0,j)); }
        let t=tile_height(&before,(0,0),5.0,false,true);
        assert!(t[&(0,0)].heights.iter().all(|h|*h==45.0));
        for j in 0..61 { assert_eq!(t[&(0,0)].height_at(60,j),t[&(1,0)].height_at(0,j)); }
        assert_eq!(t[&(1,0)].height_at(4,30),40.0);
        let no_blend=tile_height(&before,(0,0),5.0,false,false);
        assert_eq!(no_blend.len(),1);
    }
    #[test]
    fn number_fields_accept_comma_and_reject_nonfinite_or_out_of_range_values() {
        assert_eq!(parse_number("-12,35",HEIGHT_MIN,HEIGHT_MAX),Ok(-12.35));
        for text in ["NaN","inf","10001","abc"] { assert!(parse_number(text,HEIGHT_MIN,HEIGHT_MAX).is_err()); }
    }

    #[test]
    fn one_gesture_undo_redo_and_save_preserve_original_and_restore_saved_ground() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("omsi-terrain-history-{}-{stamp}",std::process::id()));
        let original=dir.join("original");let map=original.join("maps/Test");let content=dir.join("mod");
        std::fs::create_dir_all(&map).unwrap();
        std::fs::write(map.join("global.cfg"),"[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n").unwrap();
        let baseline=flat(40.0).to_bytes();
        for name in ["tile_0_0.map","tile_1_0.map"] {
            std::fs::write(map.join(name),"[version]\n14\n[terrain]\n").unwrap();
            std::fs::write(map.join(format!("{name}.terrain")),&baseline).unwrap();
        }
        let w=World::open(&original,&map.join("global.cfg"),20000101).unwrap();
        let mut editor=crate::editor::Editor::default();let at=DVec3::new(150.0,150.0,40.0);
        editor.terrain.begin(&w,at);
        editor.terrain.move_brush(&w,at,0.1);editor.terrain.move_brush(&w,at,0.1);editor.terrain.finish(&w);
        assert_eq!(editor.terrain.undo.len(),1);
        let raised=w.terrain_edits.lock()[&(0,0)].clone();assert!(raised.height_at(30,30)>40.0);
        editor.save(&w,"maps/Test/global.cfg",&content,&original).unwrap();
        let saved=content.join("maps/Test/tile_0_0.map.terrain");
        assert_eq!(Terrain::load(&saved).unwrap(),raised);
        let mut external=raised.clone();external.heights[30*61+30]+=3.0;
        w.terrain_edits.lock().insert((0,0),external.clone());
        editor.terrain.undo_redo(&w,false);
        assert_eq!(w.terrain_edits.lock()[&(0,0)],external);
        assert_eq!(editor.terrain.undo.len(),1);
        w.terrain_edits.lock().insert((0,0),raised.clone());
        editor.terrain.undo_redo(&w,false);
        assert_eq!(w.terrain_edits.lock()[&(0,0)].to_bytes(),baseline);
        editor.save(&w,"maps/Test/global.cfg",&content,&original).unwrap();
        assert_eq!(std::fs::read(&saved).unwrap(),baseline);
        editor.terrain.undo_redo(&w,true);assert_eq!(w.terrain_edits.lock()[&(0,0)],raised);
        editor.terrain.undo_redo(&w,false);
        editor.terrain.once(&w,at,Tool::Lower,1.0);assert!(!editor.terrain.can_redo());
        let before=w.terrain_edits.lock()[&(0,0)].clone();
        editor.terrain.tile=Some((0,0));editor.terrain.change_tile(&w,2.0,false);
        for j in 0..61 {let t=w.terrain_edits.lock();assert_eq!(t[&(0,0)].height_at(60,j),t[&(1,0)].height_at(0,j));}
        editor.terrain.undo_redo(&w,false);assert_eq!(w.terrain_edits.lock()[&(0,0)],before);
        assert_eq!(std::fs::read(map.join("tile_0_0.map.terrain")).unwrap(),baseline);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
