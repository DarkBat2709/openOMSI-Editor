//! Reassemble generated driving curves so lookahead and conflict tests see a
//! complete manoeuvre rather than hundreds of sub-metre native SCO segments.
use omsi_sim::traffic::{Lane,LaneBuilder};
pub fn coalesce(lanes:Vec<Lane>,groups:&[(usize,usize)])->Vec<Lane> {
    if groups.is_empty(){return lanes;}
    let mut result=Vec::new();let mut i=0;
    while i<lanes.len(){
        let first=&lanes[i];let index=first.key.map(|k|k.path as usize);
        let count=groups.iter().find(|g|Some(g.0)==index).map(|g|g.1).unwrap_or(1);
        let end=i.saturating_add(count);
        let valid=count>1 && end<=lanes.len() && lanes[i..end].iter().enumerate().all(|(n,l)| {
            l.key.map(|k|k.path as usize)==index.map(|p|p+n) && !l.reversed
                && l.kind==first.kind && l.width==first.width && l.priority==first.priority
                && l.density==first.density && l.group_density==first.group_density
                && l.no_cars==first.no_cars && l.rule_bus==first.rule_bus && l.rule_trucks==first.rule_trucks
                && l.speed_limit_kmh==first.speed_limit_kmh && l.turn==first.turn
                && l.blocks.is_empty() && l.traffic_light==first.traffic_light
                && l.no_overtaking==first.no_overtaking && l.editor_bus_only==first.editor_bus_only
                && l.crossing_problem==first.crossing_problem
                && l.editor_next.is_none() && l.editor_previous.is_none()
                && l.editor_path_base.is_empty()
                && (n==0 || lanes[i+n-1].end().distance(l.start())<0.002)
        });
        if !valid {result.push(first.clone());i+=1;continue;}
        let mut points=first.points.clone();for l in &lanes[i+1..end]{points.extend_from_slice(&l.points[1..]);}
        let curve=LaneBuilder::polyline(points,first.kind,first.width);
        let mut lane=first.clone();lane.editor_grouped_path=true;lane.points=curve.points;lane.headings=curve.headings;lane.curvature=curve.curvature;lane.dist=curve.dist;
        result.push(lane);i=end;
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use omsi_sim::traffic::{LaneKind,LaneKey};
    #[test]
    fn grouping_preserves_rules_and_rejects_gaps_or_incomplete_groups(){
        let lanes:Vec<_>=(0..4).map(|i|{let mut l=LaneBuilder::polyline(vec![DVec3::Y*i as f64,DVec3::Y*(i+1) as f64],LaneKind::Street,3.0);l.key=Some(LaneKey{tile:(0,0),id:1,path:i});l.priority=64.0;l.speed_limit_kmh=20.0;l.editor_exact_path_links=true;l}).collect();
        let joined=coalesce(lanes.clone(),&[(0,4)]);assert_eq!(joined.len(),1);assert_eq!(joined[0].length(),4.0);assert_eq!(joined[0].priority,64.0);assert_eq!(joined[0].speed_limit_kmh,20.0);
        assert_eq!(coalesce(lanes.clone(),&[(0,usize::MAX)]).len(),4);
        let mut edited=lanes.clone();edited[2].priority=192.0;assert_eq!(coalesce(edited,&[(0,4)]).len(),4);
        let mut gap=lanes;gap[2].points[0].x+=0.1;assert_eq!(coalesce(gap,&[(0,4)]).len(),4);
    }
}
