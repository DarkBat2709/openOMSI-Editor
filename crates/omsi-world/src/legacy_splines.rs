//! Old tiles place linked splines relative to the preceding road's end.
use glam::DVec2;
use omsi_geometry::SplineCurve;
use omsi_map::Tile;

/// Resolve each file before merging chrono patches and scaling to the world grid.
/// Modern records already contain explicit positions; version 0 means modern too.
pub(crate) fn resolve(tile: &mut Tile) {
    if !(1..11).contains(&tile.version) { return; }
    for i in 1..tile.splines.len() {
        let (before, after) = tile.splines.split_at_mut(i);
        let previous = &before[i - 1];
        let current = &mut after[0];
        // The old-format parser creates both links only for the continuation flag.
        if current.prev_id != previous.id || previous.next_id != current.id { continue; }
        let curve = SplineCurve::from_map(previous, DVec2::ZERO);
        current.pos = curve.end_point().to_array();
        current.heading = curve.heading_at(previous.length).rem_euclid(360.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omsi_map::MapSpline;

    #[test]
    fn curved_chain_uses_endpoint_height_and_tangent() {
        let first = MapSpline { id: 1, next_id: 2, pos: [10.0, 20.0, 3.0], heading: 0.0,
            length: std::f64::consts::FRAC_PI_2 * 20.0, radius: 20.0,
            is_h: true, delta_h: Some(4.0), ..Default::default() };
        let second = MapSpline { id: 2, prev_id: 1, next_id: 3, length: 10.0,
            grad_start: 10.0, grad_end: 10.0, ..Default::default() };
        let third = MapSpline { id: 3, prev_id: 2, ..Default::default() };
        let separate = MapSpline { id: 4, pos: [90.0, 80.0, 2.0], ..Default::default() };
        let mut tile = Tile { version: 7, splines: vec![first, second, third, separate.clone()], ..Default::default() };
        resolve(&mut tile);
        for (actual, expected) in tile.splines[1].pos.iter().zip([30.0, 40.0, 7.0]) { assert!((actual - expected).abs() < 1e-8); }
        assert!((tile.splines[1].heading - 90.0).abs() < 1e-8);
        for (actual, expected) in tile.splines[2].pos.iter().zip([40.0, 40.0, 8.0]) { assert!((actual - expected).abs() < 1e-8); }
        assert_eq!(tile.splines[3], separate);
        let resolved = tile.clone();
        resolve(&mut tile);
        assert_eq!(tile.splines, resolved.splines);
    }

    #[test]
    fn modern_explicit_positions_are_preserved() {
        for version in [0, 11, 14] {
            let mut tile = Tile { version, splines: vec![
                MapSpline { id: 1, next_id: 2, length: 50.0, ..Default::default() },
                MapSpline { id: 2, prev_id: 1, pos: [4.0, 5.0, 6.0], heading: 23.0, ..Default::default() },
            ], ..Default::default() };
            let original = tile.splines.clone();
            resolve(&mut tile);
            assert_eq!(tile.splines, original);
        }
    }

    #[test]
    fn file_loader_resolves_each_chrono_file_in_its_own_version() {
        let dir = std::env::temp_dir().join(format!("omsi-legacy-chain-{}", std::process::id()));
        let chrono = dir.join("Chrono/event");
        std::fs::create_dir_all(&chrono).unwrap();
        let path = dir.join("tile_0_0.map");
        let old = "[version]\n7\n[spline]\nroad.sli\n1\n-1\n10\n2\n20\n0\n50\n0\n10\n10\n0\n0\n[spline]\nroad.sli\n2\n0\n0\n0\n0\n0\n20\n0\n0\n0\n0\n0\n";
        let modern = "[version]\n14\n[spline]\n0\nroad.sli\n30\n0\n0\n80\n9\n90\n45\n10\n0\n0\n0\n0\n0\n0\n0\n0\n";
        for (base, patch) in [(old, modern), (modern, old)] {
            std::fs::write(&path, base).unwrap();
            std::fs::write(chrono.join("tile_0_0.map"), patch).unwrap();
            let tile = crate::map_index::read_tile(&path, std::slice::from_ref(&chrono)).unwrap();
            assert_eq!(tile.splines.len(), 3);
            let continuation = tile.splines.iter().find(|s| s.id == 2).unwrap();
            assert_eq!(continuation.pos, [10.0, 70.0, 7.0]);
            let explicit = tile.splines.iter().find(|s| s.id == 30).unwrap();
            assert_eq!(explicit.pos, [80.0, 90.0, 9.0]);
            assert_eq!(explicit.heading, 45.0);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), base);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
