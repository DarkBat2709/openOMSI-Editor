//! OMSI ground layers and north-first A8 DDS masks, shared by painting and saving.
use crate::scene::World;
use glam::DVec3;
use hashbrown::{HashMap, HashSet};
use omsi_map::GroundTex;
use omsi_texture::Image;
use std::path::{Component, Path, PathBuf};

pub type Key = ((i32, i32), usize);
const MAX_EDGE: usize = 2048;

#[derive(Clone, Debug, PartialEq)]
pub struct Mask { pub size: usize, pub alpha: Vec<f32> }

impl Mask {
    pub fn empty(size: usize) -> Result<Self, String> {
        if !(2..=MAX_EDGE).contains(&size) || !size.is_power_of_two() {
            return Err("Texture mask requires 2–2048 pixels and a power of two".into());
        }
        Ok(Self { size, alpha: vec![0.0; size * size] })
    }
    fn from_image(image: Image) -> Result<Self, String> {
        let size = image.width as usize;
        let mut out = Self::empty(size)?;
        if image.height as usize != size || image.rgba.len() != size * size * 4 || !image.has_alpha {
            return Err("Ground texture mask is not a square image with an alpha channel".into());
        }
        for (a, p) in out.alpha.iter_mut().zip(image.rgba.chunks_exact(4)) { *a = p[3] as f32 / 255.0; }
        Ok(out)
    }
    /// Stored and saved rows start at the north edge, like a conventional image.
    pub fn image(&self) -> Image {
        let mut rgba = Vec::with_capacity(self.size * self.size * 4);
        for a in &self.alpha { rgba.extend_from_slice(&[255,255,255,(a.clamp(0.0,1.0)*255.0).round() as u8]); }
        Image { width:self.size as u32, height:self.size as u32, rgba, has_alpha:true }
    }
    pub fn dds(&self) -> Vec<u8> {
        let mut bytes = vec![0u8;128]; bytes[..4].copy_from_slice(b"DDS ");
        for (at,value) in [(4,124u32),(8,0x100fu32),(12,self.size as u32),(16,self.size as u32),
            (20,self.size as u32),(76,32),(80,2),(88,8),(104,255),(108,0x1000)] {
            bytes[at..at+4].copy_from_slice(&value.to_le_bytes());
        }
        bytes.extend(self.alpha.iter().map(|a|(a.clamp(0.0,1.0)*255.0).round() as u8)); bytes
    }
    fn at(&self, local_x:f64,local_y:f64) -> f32 {
        let ts=omsi_map::tile_size();
        let x=(local_x/ts*self.size as f64).floor().clamp(0.0,(self.size-1) as f64) as usize;
        let y=((1.0-local_y/ts)*self.size as f64).floor().clamp(0.0,(self.size-1) as f64) as usize;
        self.alpha[y*self.size+x]
    }
}

pub fn layers(world:&World) -> Vec<GroundTex> {
    world.global.ground_textures.iter().cloned().chain(world.ground_texture_edits.lock().iter().cloned()).collect()
}

pub fn mask_path(world:&World,key:Key) -> Result<PathBuf,String> {
    let tile=world.tile_source(key.0.0,key.0.1).ok_or("Tile lies outside the map")?;
    let name=tile.file_name().ok_or("Tile filename missing")?.to_string_lossy();
    Ok(omsi_cfg::resolve_path(&world.map_dir,&format!("texture/map/{name}.{}.dds",key.1)))
}

pub fn read(world:&World,key:Key) -> Result<Mask,String> {
    if let Some(mask)=world.ground_paint_edits.lock().get(&key) { return Ok(mask.clone()); }
    let defs=layers(world);let def=defs.get(key.1).ok_or("Texture layer missing")?;
    let path=mask_path(world,key)?;
    if omsi_cfg::vfs::is_file(&path) {
        Mask::from_image(omsi_texture::decode_file(&path).map_err(|e|format!("{}: {e}",path.display()))?)
    } else { Mask::empty(def.mask_size() as usize) }
}

/// Object-only chrono events do not replace the ground. Check affected terrain and masks,
/// once per tile/stroke, rather than reparsing large chrono tiles on every mouse frame.
fn validate_chrono(world:&World,src:&Path,key:(i32,i32))->Result<(),String> {
    let chrono=world.chrono_dirs.read();if chrono.is_empty() {return Ok(());}
    let tile=crate::tiles::read_tile(src,&chrono).ok_or("Cannot read tile with Chrono data")?;
    if tile.terrain_from.is_some() {
        return Err(format!("Tile ({},{}): Chrono replaces the terrain; choose a time without this event for this area",key.0,key.1));
    }
    let name=src.file_name().ok_or("Tile filename missing")?.to_string_lossy();
    let prefix=format!("{}.",name.to_lowercase());
    for dir in chrono.iter() {
        let masks=omsi_cfg::resolve_path(dir,"texture/map");
        if omsi_cfg::vfs::list_dir(&masks).is_some_and(|entries|entries.iter().any(|(name,is_dir)|{
            let name=name.to_string_lossy().to_lowercase();!*is_dir&&name.starts_with(&prefix)&&name.ends_with(".dds")
        })) {
            return Err(format!("Tile ({},{}): Chrono has its own texture masks; choose a time without this event for this area",key.0,key.1));
        }
    }
    Ok(())
}

pub fn footprint(world:&World,at:DVec3,radius:f64,selected:usize,erase:bool,checked:&mut HashSet<(i32,i32)>) -> Result<HashMap<Key,Mask>,String> {
    let defs=layers(world); if selected>=defs.len() { return Err("Select a ground texture first".into()); }
    let ts=omsi_map::tile_size();let mut out=HashMap::new();
    for x in ((at.x-radius)/ts).floor() as i32..=((at.x+radius)/ts).floor() as i32 {
        for y in ((at.y-radius)/ts).floor() as i32..=((at.y+radius)/ts).floor() as i32 {
            let Some(src)=world.tile_source(x,y) else { continue; };
            let dx=(x as f64*ts-at.x).max(0.0).max(at.x-(x+1) as f64*ts);
            let dy=(y as f64*ts-at.y).max(0.0).max(at.y-(y+1) as f64*ts);
            if dx.hypot(dy)>radius { continue; }
            if !omsi_cfg::vfs::is_file(&src) { return Err("Tile file missing".into()); }
            if !checked.contains(&(x,y)) {validate_chrono(world,&src,(x,y))?;checked.insert((x,y));}
            for layer in 1..defs.len() {
                if erase && layer!=selected || !erase && layer<selected { continue; }
                let key=((x,y),layer);
                // Non-selected transparent layers need no allocation, undo snapshot or DDS.
                if layer!=selected && !world.ground_paint_edits.lock().contains_key(&key)
                    && !omsi_cfg::vfs::is_file(&mask_path(world,key)?) { continue; }
                out.insert(key,read(world,key)?);
            }
        }
    }
    Ok(out)
}

/// Lower layers become visible by clearing the covering higher layers in the same stroke.
/// The exponential approach makes coverage independent of frame rate; float working masks
/// preserve weak strokes until quantisation on upload/save.
pub fn dab(before:&HashMap<Key,Mask>,at:DVec3,radius:f64,softness:f64,selected:usize,erase:bool,amount:f64) -> HashMap<Key,Mask> {
    let ts=omsi_map::tile_size();let mut after=HashMap::new();
    for (&key,old) in before {
        if erase && key.1!=selected {continue;}
        let mut mask=old.clone();let step=ts/mask.size as f64;let mut changed=false;
        let bound=|v:f64|v.floor().clamp(0.0,mask.size as f64) as usize;
        let x0=bound((at.x-radius-key.0.0 as f64*ts)/step);let x1=bound((at.x+radius-key.0.0 as f64*ts)/step+1.0);
        let y0=bound(((key.0.1+1) as f64*ts-at.y-radius)/step);let y1=bound(((key.0.1+1) as f64*ts-at.y+radius)/step+1.0);
        for row in y0..y1 { for col in x0..x1 {
            let x=key.0.0 as f64*ts+(col as f64+0.5)*step;
            let y=(key.0.1+1) as f64*ts-(row as f64+0.5)*step;
            let w=crate::terrain_editor::falloff((x-at.x).hypot(y-at.y),radius,softness);
            if w<=0.0 { continue; }
            let target=if key.1==selected && !erase {1.0} else {0.0};
            let i=row*mask.size+col;let a=mask.alpha[i] as f64;
            let value=(a+(target-a)*(1.0-(-amount.max(0.0)*w).exp())).clamp(0.0,1.0) as f32;
            if (value-mask.alpha[i]).abs()>1e-7 {mask.alpha[i]=value;changed=true;}
        } }
        if changed {after.insert(key,mask);}
    }
    after
}

pub fn sample(world:&World,at:DVec3) -> Result<usize,String> {
    let ts=omsi_map::tile_size();let tile=((at.x/ts).floor() as i32,(at.y/ts).floor() as i32);
    let mut remaining=1.0f32;let mut best=(0,0.0f32);
    for layer in (1..layers(world).len()).rev() {
        let alpha=read(world,(tile,layer))?.at(at.x-tile.0 as f64*ts,at.y-tile.1 as f64*ts);
        let visible=alpha*remaining;if visible>best.1 {best=(layer,visible);}
        remaining*=1.0-alpha;
    }
    Ok(if remaining>=best.1 {0} else {best.0})
}

/// Add a stable OMSI-relative layer without changing the original map's base layer.
pub fn select_texture(world:&World,asset:&crate::asset_catalog::Asset,metres:f64) -> Result<usize,String> {
    let _=omsi_texture::decode_file(&asset.path).map_err(|e|e.to_string())?;
    let file=asset.file.replace('/',"\\");let defs=layers(world);
    if let Some(index)=defs.iter().position(|g|g.texture.eq_ignore_ascii_case(&file)) {return Ok(index);}
    if defs.is_empty() { return Err("The map needs a base texture in global.cfg first".into()); }
    if defs.len()>=128 { return Err("The map already has 128 ground texture layers".into()); }
    let index=defs.len();world.ground_texture_edits.lock().push(GroundTex {texture:file,detail_texture:String::new(),
        params:[9.0,(omsi_map::tile_size()/metres.clamp(0.25,100.0)) as f32,1.0]});
    Ok(index)
}

pub fn safe_map_dir(map_rel:&str) -> Result<&Path,String> {
    let path=Path::new(map_rel);
    if path.is_absolute() || path.components().any(|c|!matches!(c,Component::Normal(_))) {
        return Err("Invalid relative map path".into());
    }
    path.parent().ok_or_else(||"Map folder missing".into())
}

pub fn save(world:&World,map_rel:&str,content:&Path,original:&Path) -> Result<Vec<PathBuf>,String> {
    let dir=content.join(safe_map_dir(map_rel)?);let mut written=Vec::new();
    let additions=world.ground_texture_edits.lock().clone();
    if !additions.is_empty() {
        let out=content.join(map_rel);crate::editor::protect_output(&out,original)?;
        let bytes=if out.is_file() {std::fs::read(&out).map_err(|e|e.to_string())?}
            else {omsi_cfg::vfs::read(&world.global.path).map_err(|e|e.to_string())?};
        let (mut text,encoding)=crate::editor::decode(&bytes);
        let current=omsi_map::GlobalCfg::parse(&omsi_cfg::CfgFile::from_str("global.cfg",&text));
        if current.ground_textures.len()<world.global.ground_textures.len()
            || !current.ground_textures.iter().take(world.global.ground_textures.len()).eq(world.global.ground_textures.iter()) {
            return Err("Ground textures in global.cfg have changed; reload the map first".into());
        }
        let eol=if text.contains("\r\n") {"\r\n"} else {"\n"};
        for (i,def) in additions.iter().enumerate() {
            let index=world.global.ground_textures.len()+i;
            if let Some(existing)=current.ground_textures.get(index) {
                if existing!=def {return Err("Another texture layer occupies the new index; reload the map first".into());}
                continue;
            }
            text.push_str(&format!("{eol}[groundtex]{eol}{}{eol}{}{eol}{}{eol}{}{eol}{}{eol}",
                def.texture,def.detail_texture,def.params[0],def.params[1],def.params[2]));
        }
        if current.ground_textures.len()>world.global.ground_textures.len()+additions.len() {
            return Err("Additional unknown ground texture layers found; reload the map first".into());
        }
        std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
        crate::editor::save_copy(&out,&crate::editor::encode(&text,encoding))?;written.push(out);
    }
    let edits=world.ground_paint_edits.lock().clone();
    let mask_dir=dir.join("texture/map");
    for (key,mask) in edits {
        let src=world.tile_source(key.0.0,key.0.1).ok_or("Tile no longer exists")?;
        let name=src.file_name().ok_or("Tile filename missing")?.to_string_lossy();
        let out=mask_dir.join(format!("{name}.{}.dds",key.1));crate::editor::protect_output(&out,original)?;
        std::fs::create_dir_all(&mask_dir).map_err(|e|e.to_string())?;
        crate::editor::save_copy(&out,&mask.dds())?;written.push(out);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a8_dds_roundtrip_preserves_north_first_rows_and_soft_alpha() {
        let mut mask=Mask::empty(8).unwrap();mask.alpha[0]=1.0;mask.alpha[7*8]=0.25;
        let decoded=omsi_texture::decode_bytes(&mask.dds(),Path::new("mask.dds")).unwrap();
        let restored=Mask::from_image(decoded).unwrap();
        assert_eq!(restored.alpha[0],1.0);assert!((restored.alpha[7*8]-0.25).abs()<0.004);
        assert_eq!(restored.alpha[7],0.0);
    }
    #[test]
    fn painting_crosses_tiles_and_clears_covering_layers_without_affecting_height() {
        let mut top=Mask::empty(64).unwrap();top.alpha.fill(1.0);
        let before=[(((0,0),1),Mask::empty(64).unwrap()),(((1,0),1),Mask::empty(64).unwrap()),
            (((0,0),2),top.clone()),(((1,0),2),top)].into_iter().collect();
        let after=dab(&before,DVec3::new(300.0,150.0,99.0),20.0,100.0,1,false,2.0);
        for y in 0..64 {assert_eq!(after[&((0,0),1)].alpha[y*64+63],after[&((1,0),1)].alpha[y*64]);}
        assert!(after[&((0,0),1)].alpha[32*64+63]>0.5);
        assert!(after[&((0,0),2)].alpha[32*64+63]<0.5);
        assert_eq!(after[&((0,0),1)].alpha[0],0.0);
        let erase=dab(&after,DVec3::new(300.0,150.0,0.0),20.0,100.0,1,true,2.0);
        assert!(erase[&((0,0),1)].alpha[32*64+63]<after[&((0,0),1)].alpha[32*64+63]);
    }
    #[test]
    fn path_validation_prevents_outside_map_writes() {
        assert!(safe_map_dir("maps/Test/global.cfg").is_ok());
        for bad in ["/maps/Test/global.cfg","../global.cfg","maps/../global.cfg"] {assert!(safe_map_dir(bad).is_err());}
    }

    #[test]
    fn texture_stroke_undo_save_and_reload_leave_original_and_height_untouched() {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("omsi-ground-paint-{stamp}"));let original=dir.join("original");
        let map=original.join("maps/Test");let content=dir.join("mod");std::fs::create_dir_all(&map).unwrap();
        let global="[groundtex]\nTexture/gras.bmp\n\n6\n25\n25\n[groundtex]\nTexture/erde.bmp\n\n6\n25\n25\n[map]\n0\n0\ntile_0_0.map\n[map]\n1\n0\ntile_1_0.map\n";
        std::fs::write(map.join("global.cfg"),global).unwrap();
        for name in ["tile_0_0.map","tile_1_0.map"] {std::fs::write(map.join(name),"[version]\n14\n[terrain]\n").unwrap();}
        let world=World::open(&original,&map.join("global.cfg"),20000101).unwrap();
        let chrono=map.join("Chrono/Haltestellentausch");std::fs::create_dir_all(&chrono).unwrap();
        let object_patch="[version]\n14\n";std::fs::write(chrono.join("tile_0_0.map"),object_patch).unwrap();
        world.chrono_dirs.write().push(chrono.clone());omsi_cfg::content_changed();
        let mut terrain=crate::terrain_editor::TerrainEditor::default();terrain.mode=crate::terrain_editor::Mode::Textures;
        terrain.radius=20.0;terrain.strength=10.0;
        terrain.choose_layer(&world,1);terrain.begin(&world,DVec3::new(300.0,150.0,40.0));
        terrain.move_brush(&world,DVec3::new(300.0,150.0,40.0),0.5);terrain.finish(&world);
        let painted=read(&world,((0,0),1)).unwrap();assert!(painted.alpha.iter().any(|a|*a>0.9));
        assert!(world.terrain_edits.lock().is_empty());assert!(terrain.can_undo());
        terrain.undo_redo(&world,false);assert!(read(&world,((0,0),1)).unwrap().alpha.iter().all(|a|*a==0.0));
        terrain.undo_redo(&world,true);assert_eq!(read(&world,((0,0),1)).unwrap(),painted);
        let files=save(&world,"maps/Test/global.cfg",&content,&original).unwrap();assert_eq!(files.len(),2);
        assert_eq!(std::fs::read_to_string(map.join("global.cfg")).unwrap(),global);
        assert!(!map.join("texture/map/tile_0_0.map.1.dds").exists());
        let loaded=Mask::from_image(omsi_texture::decode_file(&content.join("maps/Test/texture/map/tile_0_0.map.1.dds")).unwrap()).unwrap();
        assert!(loaded.alpha.iter().zip(&painted.alpha).all(|(a,b)|(a-b).abs()<0.004));
        terrain.undo_redo(&world,false);save(&world,"maps/Test/global.cfg",&content,&original).unwrap();
        let erased=Mask::from_image(omsi_texture::decode_file(&content.join("maps/Test/texture/map/tile_0_0.map.1.dds")).unwrap()).unwrap();
        assert!(erased.alpha.iter().all(|a|*a==0.0));
        assert_eq!(std::fs::read_to_string(chrono.join("tile_0_0.map")).unwrap(),object_patch);
        // A real chrono terrain replacement and a tile-specific mask are still guarded.
        std::fs::write(chrono.join("tile_0_0.map"),"[version]\n14\n[terrain]\n").unwrap();
        std::fs::write(chrono.join("tile_0_0.map.terrain"),omsi_map::Terrain::flat().to_bytes()).unwrap();
        omsi_cfg::content_changed();
        let source=world.tile_source(0,0).unwrap();let error=validate_chrono(&world,&source,(0,0)).unwrap_err();
        assert!(error.contains("Chrono replaces the terrain"));
        let masks=chrono.join("texture/map");std::fs::create_dir_all(&masks).unwrap();
        std::fs::write(masks.join("tile_1_0.map.1.dds"),Mask::empty(8).unwrap().dds()).unwrap();omsi_cfg::content_changed();
        let source=world.tile_source(1,0).unwrap();let error=validate_chrono(&world,&source,(1,0)).unwrap_err();
        assert!(error.contains("own texture masks"));
        let _=std::fs::remove_dir_all(dir);
    }
}
