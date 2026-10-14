use crate::editor_splines::Key;
use hashbrown::{HashMap,HashSet};
use omsi_map::{Tile,SplineAttachment};
use serde::{Serialize,Deserialize};
use std::sync::Arc;
type TileKey=(i32,i32);
#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub interval: f64,
    pub margin: f64,
    pub start: f64,
    pub range: f64,
    pub height: f64,
    pub rotation: f64,
    pub sides: u8, // bit 1 left, bit 2 right, in the direction of the whole route
    pub connected: bool,
    pub ground: bool,
    pub junction_gap: f64,
    pub manual_gaps: Vec<[f64; 2]>,
}
impl Default for Settings {
    fn default() -> Self {
        Self { interval: 50.0, margin: 0.5, start: 0.0, range: 0.0, height: 0.0,
            rotation: 0.0, sides: 3, connected: false, ground: false, junction_gap: 8.0, manual_gaps: Vec::new() }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        for (value, low, high) in [(self.interval, 1.0, 1000.0), (self.margin, 0.0, 30.0),
            (self.start, 0.0, 100000.0), (self.range, 0.0, 100000.0),
            (self.height, -20.0, 20.0), (self.rotation, -360.0, 360.0), (self.junction_gap, 0.0, 100.0)] {
            if !value.is_finite() || !(low..=high).contains(&value) {
                return Err("Spacing or height is outside the allowed range".into());
            }
        }
        if !(1..=3).contains(&self.sides) { return Err("Choose at least one side of the road".into()); }
        for [a, b] in &self.manual_gaps {
            if !a.is_finite() || !b.is_finite() || *a < 0.0 || *b <= *a || *b > 100000.0 {
                return Err("Invalid excluded area".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: i64,
    pub start: Key,
    pub file: String,
    pub settings: Settings,
    pub rows: Vec<RowId>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RowId { pub id: i64, pub tile: TileKey, pub spline: i64, pub ordinal: usize, pub side: u8 }
#[derive(Clone, PartialEq)]
pub struct RowEdit { pub spline: i64, pub row: SplineAttachment }
#[derive(Clone)]
pub struct Source { pub tile: Arc<Tile>, pub editable: Arc<HashSet<i64>> }
#[derive(Clone, Default)]
pub struct Edits {
    pub initialized: bool,
    pub initializing: bool,
    pub initialization_error: Option<String>,
    pub sources: HashMap<TileKey, Source>,
    pub groups: Vec<Group>,
    pub changes: HashMap<TileKey, HashMap<i64, Option<RowEdit>>>,
    pub dirty_tiles: HashSet<TileKey>,
    pub save_tiles: HashSet<TileKey>,
    pub undo: Vec<Vec<Group>>,
}
impl Edits {
    pub fn overlay(&self, key: TileKey, tile: &mut Tile) {
        let Some(changes) = self.changes.get(&key) else { return; };
        tile.spline_attachments.retain(|row| !changes.contains_key(&row.id));
        let mut rows: Vec<_> = changes.values().flatten().collect();
        rows.sort_by_key(|edit| edit.row.id);
        for edit in rows {
            if let Some(index) = tile.splines.iter().position(|s| s.id == edit.spline && !s.deleted) {
                let mut row = edit.row.clone(); row.spline_index = index as i32;
                tile.spline_attachments.push(row);
            }
        }
    }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
}

