//! Commit random traffic to one reachable exit of a generated junction. Choosing
//! anew at each ring node otherwise encourages repeated laps and lane weaving.
use super::Network;
pub fn exits(net:&Network,start:usize,open:&dyn Fn(usize,usize)->bool)->Vec<Vec<usize>> {
    let owner=net.lanes[start].key;
    let same=|i:usize|net.lanes[i].editor_grouped_path && owner.zip(net.lanes[i].key).is_some_and(|(a,b)|a.tile==b.tile&&a.id==b.id);
    let mut dist=std::collections::HashMap::from([(start,0.0f32)]);
    let mut parent=std::collections::HashMap::new();let mut pending=vec![start];let mut ends=Vec::new();
    while !pending.is_empty() && dist.len()<=512 {
        let at=(0..pending.len()).min_by(|&a,&b|dist[&pending[a]].total_cmp(&dist[&pending[b]])).unwrap();let i=pending.swap_remove(at);
        let lane=&net.lanes[i];
        if i!=start && (lane.next.is_empty() || lane.next.iter().any(|&n|!same(n)&&open(i,n))) {ends.push(i);continue;}
        for &n in &lane.next {if !same(n)||!open(i,n){continue;}let d=dist[&i]+net.lanes[n].length();
            if d<*dist.get(&n).unwrap_or(&f32::INFINITY){dist.insert(n,d);parent.insert(n,i);if !pending.contains(&n){pending.push(n);}}
        }
    }
    // Prefer outlets that continue into the road network; retain terminal outlets
    // when this is an isolated test object instead of trapping cars in its ring.
    if ends.iter().any(|&i|net.reach.get(i).is_some_and(|&r|r>=super::DEAD_END)) {
        ends.retain(|&i|net.reach.get(i).is_some_and(|&r|r>=super::DEAD_END));
    }
    ends.into_iter().map(|end|{let mut route=vec![end];let mut i=end;while i!=start {i=parent[&i];route.push(i);}route.reverse();route}).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::{AiState,LaneBuilder,LaneKind,LaneKey};
    use glam::DVec3;
    #[test]
    fn commits_to_an_open_exit_without_revisiting_ring_lanes(){
        let mut lanes=Vec::new();for i in 0..6 {let mut l=LaneBuilder::polyline(vec![DVec3::new(i as f64,0.0,0.0),DVec3::new(i as f64,10.0,0.0)],LaneKind::Street,3.0);
            l.editor_grouped_path=true;l.key=Some(LaneKey{tile:(0,0),id:1,path:i});lanes.push(l);}
        lanes[0].next=vec![1];lanes[1].next=vec![2,4];lanes[2].next=vec![3,5];lanes[3].next=vec![0];
        lanes[4].no_cars=true;
        let net=Network{lanes,..Default::default()};
        for seed in 1..64 {let mut car=AiState::new(0,0.0,seed);let mut at=0;let mut seen=std::collections::HashSet::new();
            while let Some(next)=car.choose_after(&net,at){assert!(seen.insert(at));assert_ne!(next,4);at=next;}
            assert_eq!(at,5);
        }
    }
}
