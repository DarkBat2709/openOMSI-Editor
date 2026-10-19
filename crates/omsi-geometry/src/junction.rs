//! Shared junction project data and mesh construction primitives.
use glam::{DVec2,DVec3,Vec2,Vec3};
use omsi_o3d::{Mesh,Vertex,Triangle};
use serde::{Serialize,Deserialize};
use crate::roundabout::Roundabout;
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Arm {pub enabled:bool,pub angle:f64,pub width:f64,pub length:f64,pub bend:f64,pub sidewalk:f64}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct RoadSurface {pub u_start:f64,pub u_end:f64,pub reverse_v:bool,pub width_metres:f64}
impl Default for RoadSurface {fn default()->Self {Self {u_start:0.0,u_end:1.0,reverse_v:false,width_metres:4.0}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Project {
    #[serde(default, skip_serializing_if="Option::is_none")] pub roundabout:Option<Roundabout>,
    pub format:u32,pub name:String,pub arms:[Arm;4],pub corner:f64,pub texture_metres:f64,
    #[serde(default)] pub road_surface:Option<RoadSurface>,
    pub road_texture:String,pub walk_texture:String,pub markings:bool,pub left_hand:bool}

impl Default for Project {
    fn default()->Self {Self {roundabout:None,format:1,name:"Custom T-junction".into(),arms:std::array::from_fn(|i|Arm {
        enabled:i!=3,angle:[270.0,90.0,180.0,0.0][i],width:7.0,length:30.0,bend:0.0,sidewalk:0.0}),
        corner:5.0,texture_metres:4.0,road_surface:Some(RoadSurface::default()),road_texture:String::new(),walk_texture:String::new(),markings:true,left_hand:false}}
}

pub struct Built {pub mesh:Mesh,pub paths:Vec<(Vec<DVec3>,f64,i32)>,pub outline:Vec<DVec2>}
pub fn direction(angle:f64)->DVec2 {let (s,c)=angle.to_radians().sin_cos();DVec2::new(s,c)}
pub fn right(v:DVec2)->DVec2 {DVec2::new(v.y,-v.x)}

fn vertex(mesh:&mut Mesh,p:DVec3,normal:DVec3,scale:f64)->u32 {
    let index=mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex {position:Vec3::new(p.x as f32,p.z as f32,p.y as f32),normal:Vec3::new(normal.x as f32,normal.z as f32,normal.y as f32),
        uv:Vec2::new((p.x/scale) as f32,(p.y/scale) as f32)});index
}
pub fn triangle(mesh:&mut Mesh,a:DVec3,b:DVec3,c:DVec3,slot:u16,scale:f64) {
    let mut points=[a,b,c];if (c-a).cross(b-a).z<0.0 {points.swap(1,2);}
    let normal=(points[2]-points[0]).cross(points[1]-points[0]).normalize_or_zero();
    if normal.length_squared()<0.5 {return;}
    let indices=points.map(|p|vertex(mesh,p,normal,scale));mesh.triangles.push(Triangle {indices,material:slot});
}
pub fn quad(mesh:&mut Mesh,a:DVec3,b:DVec3,c:DVec3,d:DVec3,slot:u16,scale:f64) {
    triangle(mesh,a,b,c,slot,scale);triangle(mesh,a,c,d,slot,scale);
}

/// Concrete kerb stays inside the island footprint, preserving the driving corridor.
pub fn island_kerb(mesh:&mut Mesh,a:DVec2,b:DVec2,inner_a:DVec2,inner_b:DVec2,height:f64) {
    let lip=(height-0.02).max(height*0.7);
    quad(mesh,a.extend(0.0),a.extend(lip),b.extend(lip),b.extend(0.0),3,1.0);
    quad(mesh,a.extend(lip),inner_a.extend(height),inner_b.extend(height),b.extend(lip),3,1.0);
}
