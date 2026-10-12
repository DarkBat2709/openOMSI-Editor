//! OMSI ground layers and north-first A8 DDS masks, shared by painting and saving.
use crate::World;
use omsi_map::GroundTex;
use omsi_texture::Image;
use std::path::PathBuf;

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
    pub fn from_image(image: Image) -> Result<Self, String> {
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
    pub fn at(&self, local_x:f64,local_y:f64) -> f32 {
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

