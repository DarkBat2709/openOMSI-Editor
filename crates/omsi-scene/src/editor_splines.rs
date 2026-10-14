use hashbrown::{HashMap, HashSet};
use omsi_map::{MapSpline, Tile};
pub type Key = ((i32,i32), i64);
#[derive(Clone, Default)]
pub struct Edits {
    pub collected: CollectionCache,
    pub originals: HashMap<Key, MapSpline>,
    pub changed: HashMap<Key, MapSpline>,
    pub added: HashMap<Key, MapSpline>,
    pub dirty: bool,
    pub dirty_tiles: HashSet<(i32, i32)>,
    pub terrain_cache: HashMap<(i32, i32), omsi_map::Terrain>,
}

impl Edits {
    pub fn current(&self, key: Key) -> Option<MapSpline> {
        self.added.get(&key).or_else(|| self.changed.get(&key))
            .or_else(|| self.originals.get(&key)).cloned()
    }

    pub fn overlay(&self, key: (i32, i32), tile: &mut Tile) {
        for s in &mut tile.splines {
            if let Some(e) = self.changed.get(&(key, s.id)).or_else(|| self.added.get(&(key, s.id))) {
                *s = e.clone();
            }
        }
        let mut new: Vec<_> = self.added.iter()
            .filter(|((t, id), _)| *t == key && !tile.splines.iter().any(|s| s.id == *id))
            .map(|(_, s)| s.clone()).collect();
        new.sort_by_key(|s| s.id);
        tile.splines.extend(new);
    }

    pub fn for_tile(&self, key: (i32, i32)) -> HashMap<i64, MapSpline> {
        self.changed.iter().chain(self.added.iter())
            .filter(|((t, _), _)| *t == key).map(|((_, id), s)| (*id, s.clone())).collect()
    }
}


/// Only successful base/Chrono reads are memoized. Edits themselves remain live.
#[derive(Clone, Default)]
pub struct CollectionCache {
    epoch: Option<(u64, Vec<std::path::PathBuf>)>,
    tiles: HashSet<(i32, i32)>,
}
impl CollectionCache {
    pub fn prepare(&mut self, generation: u64, chrono: &[std::path::PathBuf], loaded: &[(i32, i32)]) {
        if self.epoch.as_ref().is_none_or(|(g, c)| *g != generation || c != chrono) {
            self.tiles.clear();
            self.epoch = Some((generation, chrono.to_vec()));
        }
        let loaded: HashSet<_> = loaded.iter().copied().collect();
        self.tiles.retain(|key| loaded.contains(key));
    }
    pub fn contains(&self, tile: (i32, i32)) -> bool { self.tiles.contains(&tile) }
    pub fn insert(&mut self, tile: (i32, i32)) { self.tiles.insert(tile); }
}
#[cfg(test)]
mod collection_tests {
    use super::*;
    #[test]
    fn collection_reuses_success_but_invalidates_content_chrono_and_unloaded_tiles() {
        let mut cache = CollectionCache::default();
        cache.prepare(1, &[], &[(0,0), (1,0)]);
        assert!(!cache.contains((0,0)));
        cache.insert((0,0));
        cache.prepare(1, &[], &[(0,0), (1,0)]);
        assert!(cache.contains((0,0)));
        assert!(!cache.contains((1,0)), "failed or unread tiles must retry");
        cache.prepare(2, &[], &[(0,0)]);
        assert!(!cache.contains((0,0)));
        cache.insert((0,0));
        cache.prepare(2, &["Chrono/event".into()], &[(0,0)]);
        assert!(!cache.contains((0,0)));
        cache.insert((0,0));
        cache.prepare(2, &["Chrono/event".into()], &[]);
        cache.prepare(2, &["Chrono/event".into()], &[(0,0)]);
        assert!(!cache.contains((0,0)));
    }
}
