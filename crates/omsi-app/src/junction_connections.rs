//! A builder arm in the same 3-D frame used to render its placed object.
use glam::DVec3;
#[derive(Clone,Copy,Debug)]
pub struct Pose {pub at:DVec3,pub heading:f64,pub tilt:[f64;2]}
impl Pose {
    pub fn rotation(self)->glam::DMat4 {
        omsi_geometry::object_rotation(omsi_geometry::map_rotation([self.heading,self.tilt[0],self.tilt[1]])).as_dmat4()
    }
    pub fn point(self,p:DVec3)->DVec3 {self.at+self.rotation().transform_vector3(p)}
    pub fn local(self,p:DVec3)->DVec3 {self.rotation().inverse().transform_vector3(p-self.at)}
    pub fn port(self,p:DVec3,outward:f64)->Result<Port,String> {
        let (s,c)=outward.to_radians().sin_cos();let r=self.rotation();
        let forward=r.transform_vector3(DVec3::new(s,c,0.0));
        let right=r.transform_vector3(DVec3::new(c,-s,0.0));
        let up=r.transform_vector3(DVec3::Z);
        if !forward.is_finite()||forward.truncate().length()<0.2||up.z<0.2 {return Err("Junction is too steep for a road connection".into());}
        Ok(Port{point:self.point(p),outward:forward.x.atan2(forward.y).to_degrees(),
            grade:forward.z/forward.truncate().length()*100.0,
            cant:-right.z/right.truncate().length()*100.0,right,up})
    }
}
#[derive(Clone,Copy,Debug)]
pub struct Port {pub point:DVec3,pub outward:f64,pub grade:f64,pub cant:f64,right:DVec3,up:DVec3}
impl Port {
    /// Target profiles look inward, so their right axis is opposite the arm's.
    pub fn surface(self,x:f64,z:f64)->DVec3 {self.point-self.right*x+self.up*z}
}
