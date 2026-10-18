//! Route new catalogue/copy placements to their current tile without losing prior saves.
use super::*;
type Tile = (i32, i32);
pub(super) struct PlacementPlan {
    pub records: HashMap<Tile, Vec<PlacedRecord>>,
    pub rules: HashMap<Tile, Vec<(i64, Option<String>)>>,
}
impl Editor {
    pub(super) fn plan_placements(&self, world: &World) -> Result<PlacementPlan, String> {
        let mut plan = PlacementPlan { records: HashMap::new(), rules: HashMap::new() };
        let history = self.saved_placement_tiles.lock().unwrap();
        let mut validated = hashbrown::HashSet::new();
        for a in self.added.iter().filter(|a| a.standalone) {
            let position = a.base + a.moved;
            let size = omsi_map::tile_size();
            // A deleted object may have been dragged outside the map. Only its old
            // records need removing; its current position must not block saving.
            let tile = if a.deleted { a.tile } else {
                if !position.is_finite() { return Err(format!("Object {} has an invalid position", a.id)); }
                let tile = ((position.x / size).floor() as i32, (position.y / size).floor() as i32);
                if validated.insert(tile) {
                    let src = world.tile_source(tile.0, tile.1).ok_or_else(||
                        format!("Object {} is outside the map at tile ({}, {}); move it back or create that tile", a.id, tile.0, tile.1))?;
                    let target = omsi_map::Tile::load(&src).map_err(|e| format!("{}: {e}", src.display()))?;
                    if target.version != 0 && target.version < 14 {
                        return Err("Object placement requires tile version 14".into());
                    }
                }
                tile
            };
            let mut roots = omsi_cfg::content_roots(); roots.push(world.root.clone());
            let file = roots.iter().filter_map(|root| a.sco.strip_prefix(root).ok().map(|p| (root.components().count(), p)))
                .max_by_key(|(depth, _)| *depth).map(|(_, p)| p).unwrap_or(&a.sco)
                .to_string_lossy().replace('/', "\\");
            let mut record = PlacedRecord { id: a.id, file,
                pos: position - DVec3::new(tile.0 as f64 * size, tile.1 as f64 * size, 0.0),
                heading: a.base_heading + a.turned, tilt: a.tilt, strings: a.strings.clone(), deleted: a.deleted };
            let rules = if a.deleted { None } else {
                let ot = world.editor_object_type(&a.sco.to_string_lossy())?;
                if !ot.sco.absolute_height() {
                    record.pos.z -= world.ground_terrain(position.x, position.y).unwrap_or(a.ground_height);
                }
                crate::junction_builder::roundabout_rules(&a.sco)?
            };
            // Rebuild every former destination from its cached source, removing this ID
            // there. Keeping history after deletion also makes restore and undo safe.
            let mut previous = history.get(&a.id).cloned().unwrap_or_default();
            previous.insert(a.tile);
            for old in previous.into_iter().filter(|old| *old != tile) {
                let mut removed = record.clone(); removed.deleted = true;
                plan.records.entry(old).or_default().push(removed);
                plan.rules.entry(old).or_default().push((a.id, None));
            }
            plan.records.entry(tile).or_default().push(record);
            plan.rules.entry(tile).or_default().push((a.id, rules));
        }
        Ok(plan)
    }
}

#[cfg(test)]
mod tests;
