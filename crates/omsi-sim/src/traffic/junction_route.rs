//! Commit random traffic to one reachable exit of a generated junction. Choosing
//! anew at each ring node otherwise encourages repeated laps and lane weaving.
use super::Network;
pub fn exits(net:&Network,start:usize,open:&dyn Fn(usize,usize)->bool)->Vec<Vec<usize>> {
    // Generated junctions already provide a complete route through their owner.
    // Their intermediate lane-change sections need not be marked as ring lanes.
    if net.lanes[start].ring && !net.lanes[start].editor_grouped_path { return ring_exits(net, start, open); }
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
// Hand-drawn sections have different owners. Follow the detected ring and include
// the first non-ring lane in the plan, so reaching an exit cannot choose another lap.
fn ring_exits(net: &Network, start: usize, open: &dyn Fn(usize, usize) -> bool) -> Vec<Vec<usize>> {
    let mut distances = std::collections::HashMap::from([(start, 0.0f32)]);
    let mut parents = std::collections::HashMap::new();
    let mut pending = vec![start];
    let mut ends = Vec::new();
    while !pending.is_empty() && distances.len() <= 512 {
        let at = (0..pending.len()).min_by(|&a, &b| distances[&pending[a]].total_cmp(&distances[&pending[b]])).unwrap();
        let lane = pending.swap_remove(at);
        if !net.lanes[lane].ring { ends.push(lane); continue; }
        for &next in &net.lanes[lane].next {
            if !open(lane, next) { continue; }
            let distance = distances[&lane] + net.lanes[next].length();
            if distance < *distances.get(&next).unwrap_or(&f32::INFINITY) {
                distances.insert(next, distance);
                parents.insert(next, lane);
                if !pending.contains(&next) { pending.push(next); }
            }
        }
    }
    if ends.iter().any(|&i| net.reach.get(i).is_some_and(|&r| r >= super::DEAD_END)) {
        ends.retain(|&i| net.reach.get(i).is_some_and(|&r| r >= super::DEAD_END));
    }
    ends.into_iter().map(|end| {
        let mut route = vec![end];
        let mut lane = end;
        while lane != start { lane = parents[&lane]; route.push(lane); }
        route.reverse();
        route
    }).collect()
}
/// Avoid a random return to the incoming road when another legal outlet exists.
/// Keep it as fallback: a restricted or unfinished network can have no other exit.
pub fn prefer_other_arm(net: &Network, entry: Option<usize>, routes: &mut Vec<Vec<usize>>) {
    let Some(incoming) = entry.and_then(|i| net.lanes.get(i)) else { return };
    let is_return = |route: &Vec<usize>| {
        let Some(outgoing) = route.last().and_then(|&i| net.lanes.get(i)) else { return false };
        let separation = (incoming.end() - outgoing.start()).truncate().length();
        let arm_width = (incoming.width + outgoing.width) as f64 + 6.0;
        let heading = super::wrap_deg(outgoing.end_heading() - incoming.start_heading()).abs();
        separation <= arm_width && heading > 120.0
    };
    if routes.iter().any(|route| !is_return(route)) {
        routes.retain(|route| !is_return(route));
    }
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
    #[test]
    fn hand_drawn_ring_commits_to_an_exit_across_distinct_path_owners() {
        let mut lanes = Vec::new();
        for i in 0..6 {
            let mut lane = LaneBuilder::polyline(vec![DVec3::new(i as f64, 0.0, 0.0), DVec3::new(i as f64, 10.0, 0.0)], LaneKind::Street, 3.0);
            lane.key = Some(LaneKey { tile: (0, 0), id: 100 + i as i64, path: 0 });
            lane.ring = i < 4;
            lanes.push(lane);
        }
        lanes[0].next = vec![1]; lanes[1].next = vec![2, 4];
        lanes[2].next = vec![3, 5]; lanes[3].next = vec![0];
        lanes[4].no_cars = true;
        let net = Network { lanes, ..Default::default() };
        for seed in 1..128 {
            let mut car = AiState::new(0, 0.0, seed);
            let mut at = 0;
            let mut seen = std::collections::HashSet::new();
            while let Some(next) = car.choose_after(&net, at) {
                assert!(seen.insert(at), "repeated ring lane for seed {seed}");
                assert_ne!(next, 4);
                at = next;
            }
            assert_eq!(at, 5);
        }
    }

    fn return_choice_network(other_exit_open: bool) -> Network {
        let points = [(-10.0,0.0),(0.0,-10.0),(10.0,0.0),(0.0,10.0)];
        let v = |p:(f64,f64)| DVec3::new(p.0,p.1,0.0);
        let mut lanes = Vec::new();
        for i in 0..4 {
            let mut l = LaneBuilder::polyline(vec![v(points[i]),v(points[(i+1)%4])],LaneKind::Street,3.0);
            l.ring=true; l.next=vec![(i+1)%4]; lanes.push(l);
        }
        lanes.push(LaneBuilder::polyline(vec![v((-10.0,0.0)),v((-40.0,0.0))],LaneKind::Street,3.0));
        lanes.push(LaneBuilder::polyline(vec![v((10.0,0.0)),v((40.0,0.0))],LaneKind::Street,3.0));
        lanes.push(LaneBuilder::polyline(vec![v((-40.0,3.0)),v((-10.0,3.0))],LaneKind::Street,3.0));
        lanes[3].next.push(4); lanes[1].next.push(5); lanes[6].next=vec![0];
        lanes[5].no_cars=!other_exit_open;
        Network {lanes,..Default::default()}
    }
    #[test]
    fn ring_prefers_another_arm_over_returning_to_its_entry() {
        let net=return_choice_network(true);
        for seed in 1..128 {
            let mut car=AiState::new(6,0.0,seed);
            let mut at=6;
            for _ in 0..8 {match car.choose_after(&net,at) {Some(next)=>at=next,None=>break}}
            assert_eq!(at,5,"returned to entrance for seed {seed}");
        }
    }
    #[test]
    fn ring_keeps_return_when_all_other_exits_are_closed() {
        let net=return_choice_network(false);
        let mut car=AiState::new(6,0.0,2);
        let mut at=6;
        for _ in 0..8 {match car.choose_after(&net,at) {Some(next)=>at=next,None=>break}}
        assert_eq!(at,4);
    }

}
