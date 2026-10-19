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
/// Editor changes address the complete grouped curve by its first native index.
/// Reassemble before applying them: deleting the first segment beforehand would
/// expose the remainder, and bending it would break the group's continuity.
/// Keep legacy edits to interior segments when no whole-curve edit exists.
pub fn restore(lanes: Vec<Lane>, groups: &[(usize, usize)], rules: &[omsi_map::MapRule]) -> Vec<Lane> {
    let groups: Vec<_> = groups.iter().copied().filter(|&(start, count)| {
        let whole = crate::editor_paths::read(rules, start as u16, false)
            != crate::editor_paths::Change::default();
        whole || !rules.iter().any(|r| !r.kill
            && r.kind.starts_with(crate::editor_paths::PREFIX)
            && r.path_index as i64 > start as i64
            && (r.path_index as i64) < start.saturating_add(count) as i64
            && crate::editor_paths::read(rules, r.path_index as u16, false)
                != crate::editor_paths::Change::default())
    }).collect();
    coalesce(lanes, &groups).into_iter().filter_map(|mut lane| {
        crate::editor_paths::from_rules(&mut lane, rules).then_some(lane)
    }).collect()
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
    #[test]
    fn reload_applies_saved_deletion_and_shape_to_whole_curve() {
        let mut text = "[editor_path_group]\n0\n3\n".to_string();
        for y in [0,10,20] {text.push_str(&format!("[path]\n0\n{y}\n0\n0\n0\n10\n0\n0\n0\n3\n0\n1\n"));}
        let sco = omsi_scenery::sco::SceneryObject::parse(&omsi_cfg::CfgFile::from_str("test.sco", &text));
        let load = |rules: &[omsi_map::MapRule]| crate::lanes::object_lanes(
            &sco, DVec3::ZERO, [0.;3], None, (0,0), 1, rules);
        let rule = |path, kind: &str, value| omsi_map::MapRule {
            path_index: path, kind: kind.into(), value, ..Default::default()
        };
        let base = load(&[]); assert_eq!(base.len(), 1); assert_eq!(base[0].length(), 30.);
        let deletion = rule(0, "editor_lane_deleted_forward", 1.);
        assert!(load(&[deletion.clone()]).is_empty());
        // Extra deletions made after a broken reload must not resurrect fragments.
        assert!(load(&[deletion, rule(1,"editor_lane_deleted_forward",1.)]).is_empty());
        let shaped = load(&[rule(0,"editor_lane_1_x_forward",4.)]);
        let mut expected = base[0].clone();
        crate::editor_paths::apply_shape(&mut expected, [[4.,0.,0.],[0.;3]]);
        assert_eq!(shaped.len(),1); assert_eq!(shaped[0].points,expected.points);
        assert!(shaped[0].editor_grouped_path);
        let legacy = load(&[rule(1,"editor_lane_deleted_forward",1.)]);
        assert_eq!(legacy.len(),2); assert_eq!(legacy[1].key.unwrap().path,2);
    }

    #[test]
    #[ignore = "read-only check of a supplied saved map and generated SCO"]
    fn saved_roundabout_diagnostic() {
        let read = |name: &str| {
            let path = std::env::var(name).expect(name);
            let bytes = std::fs::read(&path).unwrap();
            let text = if bytes.starts_with(&[255,254]) {
                String::from_utf16(&bytes[2..].chunks_exact(2).map(|b|u16::from_le_bytes([b[0],b[1]])).collect::<Vec<_>>()).unwrap()
            } else {String::from_utf8(bytes).unwrap()};
            omsi_cfg::CfgFile::from_str(path, &text)
        };
        let tile = omsi_map::Tile::parse(&read("DIAG_TILE"));
        let id: i64 = std::env::var("DIAG_OBJECT").unwrap().parse().unwrap();
        let object = tile.objects.iter().find(|o|o.id==id).unwrap();
        let sco = omsi_scenery::sco::SceneryObject::parse(&read("DIAG_SCO"));
        let load = |rules: &[omsi_map::MapRule]| crate::lanes::object_lanes(&sco,DVec3::ZERO,[0.;3],None,(0,0),id,rules);
        let original = load(&[]); let saved = load(&object.rules);
        let deleted = original.iter().filter(|l|crate::editor_paths::read(&object.rules,l.key.unwrap().path,l.reversed).deleted).count();
        println!("original curves={}, saved deletions={}, restored curves={}",original.len(),deleted,saved.len());
        assert_eq!(saved.len(),original.len()-deleted);
    }

}
