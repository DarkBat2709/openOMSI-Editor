//! Read-only connection checks. A visible seam is not proof of a stored connection.
use glam::{DVec2,DVec3};
use omsi_map::MapSpline;
use omsi_geometry::SplineCurve;
use std::collections::HashMap;

pub type Key=((i32,i32),i64);
pub const TOLERANCE:f64=0.15;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Status {Broken,Open,Unknown,Good}
impl Status {
    pub fn color(self)->[f32;3]{match self{Self::Broken=>[1.0,0.12,0.12],Self::Open=>[1.0,0.8,0.05],Self::Unknown=>[0.65,0.65,0.7],Self::Good=>[0.1,1.0,0.3]}}
    pub fn name(self)->&'static str{match self{Self::Broken=>"ERROR",Self::Open=>"OPEN",Self::Unknown=>"UNCHECKED",Self::Good=>"CONNECTED"}}
    fn rank(self)->u8{match self{Self::Broken=>0,Self::Open=>1,Self::Unknown=>2,Self::Good=>3}}
}
#[derive(Clone)]
pub struct Object {pub id:i64,pub deleted:bool,pub ports:Option<Vec<DVec3>>}
#[derive(Clone,Debug)]
pub struct Entry {pub key:Key,pub end:usize,pub at:DVec3,pub status:Status,pub reason:String,pub file:String}
#[derive(Default)]
pub struct Report {pub entries:Vec<Entry>,pub roads:usize,pub unread_tiles:Vec<(i32,i32)>,counts:[usize;4],marker_cells:HashMap<(i32,i32),Vec<usize>>}
impl Report {
    pub fn counts(&self)->[usize;4]{self.counts}
    pub fn visible_len(&self,good:bool)->usize{self.entries.len()-if good{0}else{self.counts[3]}}
    pub fn nearby(&self,at:DVec3,good:bool)->impl Iterator<Item=&Entry>{
        let cx=(at.x/500.0).floor()as i32;let cy=(at.y/500.0).floor()as i32;
        (-1..=1).flat_map(move|dx|(-1..=1).map(move|dy|(cx+dx,cy+dy)))
            .flat_map(move|key|self.marker_cells.get(&key).into_iter().flatten())
            .map(|&i|&self.entries[i]).filter(move|e|(good||e.status!=Status::Good)&&e.at.distance(at)<500.0)
    }
}
/// Total editor movements are relative to this baseline even after a map save.
pub fn restore_object_baseline(tile:&mut omsi_map::Tile,baseline:&omsi_map::Tile,edited:impl Fn(i64)->bool){
    let originals:Vec<_>=baseline.objects.iter().filter(|o|edited(o.id)).cloned().collect();
    let ids:std::collections::HashSet<_>=originals.iter().map(|o|o.id).collect();
    tile.objects.retain(|o|!ids.contains(&o.id));tile.objects.extend(originals);
}
fn points(tile:(i32,i32),s:&MapSpline,tile_size:f64)->[DVec3;2]{
    let c=SplineCurve::from_map(s,DVec2::new(tile.0 as f64,tile.1 as f64)*tile_size);[c.point_at(0.0),c.end_point()]
}
#[derive(Clone,Copy)]
enum Target {Road(usize),Object(usize)}

pub fn inspect(roads:&[(Key,MapSpline)],objects:&[Object],tile_size:f64,unread_tiles:Vec<(i32,i32)>)->Report{
    let mut ids:HashMap<i64,Vec<Target>>=HashMap::new();
    for(i,(_,s))in roads.iter().enumerate(){ids.entry(s.id).or_default().push(Target::Road(i));}
    for(i,o)in objects.iter().enumerate(){ids.entry(o.id).or_default().push(Target::Object(i));}
    let ends:Vec<_>=roads.iter().map(|(k,s)|points(k.0,s,tile_size)).collect();
    let mut report=Report{unread_tiles,..Default::default()};
    let mut occupied:HashMap<(i64,usize),Vec<usize>>=HashMap::new();
    for(i,(key,s))in roads.iter().enumerate().filter(|(_,(_,s))|!s.deleted){
        report.roads+=1;
        for(end,link)in [s.prev_id,s.next_id].into_iter().enumerate(){
            let at=ends[i][end];let mut port=None;
            let(status,reason)=if !at.is_finite()||!s.length.is_finite()||s.length<=0.0 {
                (Status::Broken,"Invalid spline geometry".into())
            }else if ids.get(&s.id).is_some_and(|v|v.len()!=1){
                (Status::Broken,"Own ID assigned more than once".into())
            }else if link==0 {
                (Status::Open,"No reference – may be an intentional end".into())
            }else if link==s.id {
                (Status::Broken,"Reference to itself".into())
            }else{match ids.get(&link){
                None if !report.unread_tiles.is_empty()=>(Status::Unknown,format!("ID {link} not found; check incomplete")),
                None=>(Status::Broken,format!("Target ID {link} missing")),
                Some(v) if v.len()!=1=>(Status::Broken,format!("Target ID {link} assigned more than once")),
                Some(v)=>match v[0]{
                    Target::Road(j)=>{let other=&roads[j].1;
                        if other.deleted {(Status::Broken,format!("Target spline {link} is deleted"))}
                        else{
                            let reciprocal:Vec<_>=[other.prev_id,other.next_id].into_iter().enumerate().filter(|(_,id)|*id==s.id).map(|(e,_)|e).collect();
                            if reciprocal.is_empty(){(Status::Broken,format!("Spline {link} does not link back"))}
                            else{let gap=reciprocal.iter().map(|e|at.distance(ends[j][*e])).filter(|d|d.is_finite()).min_by(f64::total_cmp).unwrap_or(f64::INFINITY);
                                if gap>TOLERANCE {(Status::Broken,format!("Spline {link}: endpoint distance {gap:.2} m"))}
                                else{(Status::Good,format!("Spline {link}: mutual reference, distance {gap:.3} m"))}}
                        }
                    }
                    Target::Object(j)=>{let o=&objects[j];
                        if o.deleted {(Status::Broken,format!("Target object {link} is deleted"))}
                        else if let Some(ports)=&o.ports {
                            let nearest=ports.iter().enumerate().map(|(p,q)|(p,at.distance(*q))).filter(|(_,d)|d.is_finite()).min_by(|a,b|a.1.total_cmp(&b.1));
                            match nearest {Some((p,gap))if gap<=TOLERANCE=>{port=Some((link,p));(Status::Good,format!("Junction {link}: arm {}, distance {gap:.3} m",p+1))},
                                Some((_,gap))=>(Status::Broken,format!("Junction {link}: distance to nearest arm {gap:.2} m")),
                                None=>(Status::Unknown,format!("Object {link}: no verifiable arms"))}
                        }else{(Status::Unknown,format!("Object {link} exists; connection geometry unchecked"))}
                    }
                }
            }};
            if let Some(p)=port{occupied.entry(p).or_default().push(report.entries.len());}
            report.entries.push(Entry{key:*key,end,at,status,reason,file:s.file.clone()});
        }
    }
    for indices in occupied.values().filter(|v|v.len()>1){for &i in indices{
        report.entries[i].status=Status::Broken;report.entries[i].reason="Multiple spline ends occupy the same junction arm".into();
    }}
    report.entries.sort_by_key(|e|(e.status.rank(),e.key,e.end));
    for(i,e)in report.entries.iter().enumerate(){
        report.counts[e.status.rank()as usize]+=1;
        if e.at.is_finite(){report.marker_cells.entry(((e.at.x/500.0).floor()as i32,(e.at.y/500.0).floor()as i32)).or_default().push(i);}
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    fn road(id:i64,y:f64,prev:i64,next:i64)->(Key,MapSpline){(((0,0),id),MapSpline{id,pos:[0.0,y,0.0],length:10.0,prev_id:prev,next_id:next,..Default::default()})}
    fn end(r:&Report,id:i64,e:usize)->&Entry{r.entries.iter().find(|x|x.key.1==id&&x.end==e).unwrap()}
    #[test]fn reciprocal_links_require_matching_positions(){
        let a=road(1,0.0,0,2);let mut b=road(2,10.0,1,0);
        let r=inspect(&[a.clone(),b.clone()],&[],300.0,vec![]);assert_eq!(end(&r,1,1).status,Status::Good);assert_eq!(end(&r,2,0).status,Status::Good);assert_eq!(end(&r,1,0).status,Status::Open);
        b.1.pos[2]=0.5;assert_eq!(end(&inspect(&[a.clone(),b.clone()],&[],300.0,vec![]),1,1).status,Status::Broken);
        b.1.pos[2]=0.0;b.1.prev_id=0;assert_eq!(end(&inspect(&[a,b],&[],300.0,vec![]),1,1).status,Status::Broken);
    }
    #[test]fn missing_deleted_duplicate_and_incomplete_are_distinct(){
        let a=road(1,0.0,0,2);let mut b=road(2,10.0,1,0);b.1.deleted=true;
        assert_eq!(end(&inspect(&[a.clone(),b],&[],300.0,vec![]),1,1).status,Status::Broken);
        assert_eq!(end(&inspect(&[a.clone()],&[],300.0,vec![]),1,1).status,Status::Broken);
        assert_eq!(end(&inspect(&[a.clone()],&[],300.0,vec![(1,0)]),1,1).status,Status::Unknown);
        let b=road(2,10.0,1,0);assert_eq!(end(&inspect(&[a,b.clone(),b],&[],300.0,vec![]),1,1).status,Status::Broken);
    }
    #[test]fn custom_junctions_generic_objects_and_double_occupancy(){
        let a=road(1,0.0,0,99);let mut o=Object{id:99,deleted:false,ports:None};
        assert_eq!(end(&inspect(&[a.clone()],&[o.clone()],300.0,vec![]),1,1).status,Status::Unknown);
        o.ports=Some(vec![DVec3::new(0.0,10.0,0.0)]);
        assert_eq!(end(&inspect(&[a.clone()],&[o.clone()],300.0,vec![]),1,1).status,Status::Good);
        assert_eq!(end(&inspect(&[a,road(2,0.0,0,99)],&[o],300.0,vec![]),1,1).status,Status::Broken);
    }
    #[test]fn saved_object_moves_are_applied_once_without_hiding_duplicate_ids(){
        let old=omsi_map::MapObject{id:99,pos:[10.0,0.0,0.0],..Default::default()};
        let baseline=omsi_map::Tile{objects:vec![old.clone(),old.clone()],..Default::default()};
        let mut moved=old.clone();moved.pos[0]=15.0;
        let mut tile=omsi_map::Tile{objects:vec![moved,omsi_map::MapObject{id:100,..Default::default()}],..Default::default()};
        restore_object_baseline(&mut tile,&baseline,|id|id==99);
        let matches:Vec<_>=tile.objects.iter().filter(|o|o.id==99).collect();
        assert_eq!(matches.len(),2);assert_eq!(matches[0].pos[0]+5.0,15.0);assert!(tile.objects.iter().any(|o|o.id==100));
    }
    #[test]fn nearby_markers_follow_the_filter_and_ignore_far_ends(){
        let r=inspect(&[road(1,0.0,0,2),road(2,10.0,1,0),road(3,2000.0,0,0)],&[],300.0,vec![]);
        assert_eq!(r.counts(),[0,4,0,2]);assert_eq!(r.visible_len(false),4);
        assert_eq!(r.nearby(DVec3::ZERO,false).count(),2);assert_eq!(r.nearby(DVec3::ZERO,true).count(),4);
    }
    #[test]fn tile_boundaries_and_curves_use_world_endpoints(){
        let a=(((0,0),1),MapSpline{id:1,pos:[299.0,0.0,0.0],heading:90.0,length:10.0,radius:20.0,next_id:2,..Default::default()});
        let p=points(a.0.0,&a.1,300.0)[1];let b=(((1,0),2),MapSpline{id:2,pos:[p.x-300.0,p.y,p.z],length:10.0,prev_id:1,..Default::default()});
        let r=inspect(&[a,b],&[],300.0,vec![]);assert_eq!(end(&r,1,1).status,Status::Good);
    }
}
