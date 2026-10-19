//! Editable chain of quartic curves: three form handles plus both endpoints.
use glam::DVec3;
pub const ENDPOINT:usize=10000;
#[derive(Clone,Default)]
pub struct FreePath {pub points:Vec<DVec3>,pub bends:Vec<Option<[DVec3;3]>>}
pub fn elevate(a:DVec3,p:DVec3,q:DVec3,b:DVec3)->[DVec3;3]{[a*0.25+p*0.75,(p+q)*0.5,q*0.75+b*0.25]}
pub fn point(a:DVec3,c:[DVec3;3],b:DVec3,t:f64)->DVec3 {
    let u=1.0-t;a*u.powi(4)+c[0]*4.0*u.powi(3)*t+c[1]*6.0*u*u*t*t+c[2]*4.0*u*t.powi(3)+b*t.powi(4)
}
impl FreePath {
    pub fn add(&mut self,p:DVec3)->Result<(),String>{
        if !p.is_finite()||self.points.len()>=128{return Err("Invalid point or too many sections".into());}
        if self.points.last().is_some_and(|q|q.truncate().distance(p.truncate())<1.0){return Err("Points must be at least 1 m apart".into());}
        self.points.push(p);self.bends.resize(self.points.len().saturating_sub(1),None);Ok(())
    }
    pub fn undo(&mut self){self.points.pop();self.bends.truncate(self.points.len().saturating_sub(1));}
    pub fn direction(&self,i:usize)->DVec3 {
        (self.points[(i+1).min(self.points.len()-1)]-self.points[i.saturating_sub(1)]).normalize_or_zero()
    }
    pub fn controls(&self,i:usize)->[DVec3;3]{
        self.bends[i].unwrap_or_else(||{let a=self.points[i];let b=self.points[i+1];let d=a.distance(b)/3.0;elevate(a,a+self.direction(i)*d,b-self.direction(i+1)*d,b)})
    }
    pub fn handles(&self)->Vec<(usize,DVec3)>{
        let mut out:Vec<_>=self.points.iter().enumerate().map(|(i,&p)|(ENDPOINT+i,p)).collect();
        for i in 0..self.bends.len(){out.extend(self.controls(i).into_iter().enumerate().map(|(k,p)|(i*3+k,p)));}out
    }
    pub fn move_handle(&mut self,index:usize,p:DVec3){
        if !p.is_finite(){return;}
        if index>=ENDPOINT {if let Some(q)=self.points.get_mut(index-ENDPOINT){*q=p;}return;}
        let i=index/3;if i>=self.bends.len(){return;}let mut c=self.controls(i);c[index%3]=p;self.bends[i]=Some(c);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_controls_and_shared_endpoints_survive_chain_edits(){
        let mut p=FreePath::default();for q in [DVec3::ZERO,DVec3::Y*20.0,DVec3::new(10.0,40.0,0.0)]{p.add(q).unwrap();}
        assert_eq!(p.handles().len(),9);let old=point(p.points[0],p.controls(0),p.points[1],0.5);
        p.move_handle(1,DVec3::new(8.0,10.0,0.0));assert!(point(p.points[0],p.controls(0),p.points[1],0.5).distance(old)>1.0);
        p.move_handle(ENDPOINT+1,DVec3::new(1.0,20.0,0.0));assert_eq!(point(p.points[0],p.controls(0),p.points[1],1.0),point(p.points[1],p.controls(1),p.points[2],0.0));
        p.undo();assert_eq!(p.handles().len(),5);assert!(p.add(p.points[1]).is_err());
    }
    #[test]
    fn degree_elevation_keeps_existing_cubic_connectors(){
        let [a,p,q,b]=[DVec3::ZERO,DVec3::Y*10.0,DVec3::new(5.0,20.0,0.0),DVec3::new(5.0,30.0,0.0)];
        for i in 0..101 {let t=i as f64/100.0;let u=1.0-t;assert!(point(a,elevate(a,p,q,b),b,t).distance(a*u*u*u+p*3.0*u*u*t+q*3.0*u*t*t+b*t*t*t)<1e-10);}
    }
}
