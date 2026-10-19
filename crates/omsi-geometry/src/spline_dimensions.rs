//! Validated numeric road dimensions and construction-plane geometry.
use glam::{DVec2,DVec3};
use crate::SplineCurve;
use omsi_map::MapSpline;
pub fn validate(v:[f64;4])->Result<(),String> {
    if v.iter().any(|n|!n.is_finite()) || !(0.5..=500.0).contains(&v[0])
        || (v[1]!=0.0 && !(0.5..=100000.0).contains(&v[1].abs())) || v[2].abs()>20.0
        || !(-1000.0..=10000.0).contains(&v[3]) {
        return Err("Length: 0.5–500 m; radius: 0 or ±0.5–100000 m; gradient: ±20%; height: -1000–10000 m".into());
    }
    Ok(())
}
pub fn reshape(s:&MapSpline,v:[f64;4])->Result<MapSpline,String> {
    validate(v)?;
    let mut out=s.clone();
    if !s.length.is_finite() || s.length<=0.0 {return Err("Invalid spline length".into());}
    for t in out.profile_transitions.iter_mut().flatten() {t.station*=v[0]/s.length;t.span*=v[0]/s.length;}
    out.length=v[0];out.radius=v[1];out.grad_start=v[2];out.grad_end=v[2];out.pos[2]=v[3];
    out.delta_h=Some(v[0]*v[2]/100.0);out.is_h=true;out.terrain_align=None;out.terrain_align_flag=false;
    Ok(out)
}
/// A real ray/plane intersection, including a camera below the terrain.
pub fn plane_hit(origin: DVec3, direction: DVec3, z: f64) -> Option<DVec3> {
    if !z.is_finite() || direction.z.abs() < 1e-9 {
        return None;
    }
    let distance = (z - origin.z) / direction.z;
    (distance > 0.01 && distance <= 2000.0).then(|| origin + direction * distance)
}
/// Keep exact end height independent of the endpoint gradients (OMSI spline_h).
pub fn height_profile(s: &omsi_map::MapSpline, v: [f64; 5]) -> Result<omsi_map::MapSpline, String> {
    if v.iter().any(|v| !v.is_finite())
        || !(0.5..=5000.0).contains(&s.length)
        || v[..2].iter().any(|v| !(-1000.0..=10000.0).contains(v))
        || v[2..4].iter().any(|v| v.abs() > 20.0)
    {
        return Err("Invalid height or gradient".into());
    }
    let mut result = s.clone();
    result.pos[2] = v[0];
    result.delta_h = Some(v[1] - v[0]);
    result.grad_start = v[2];
    result.grad_end = v[3];
    result.is_h = true;
    result.terrain_align = None;
    result.terrain_align_flag = false;
    let curve = crate::SplineCurve::from_map(&result, glam::DVec2::ZERO);
    // The gradient is quadratic: test its endpoints and the interior extremum exactly.
    let a = curve.slope_at(0.0);
    let b = curve.slope_at(s.length);
    let mid = curve.slope_at(s.length * 0.5);
    let quadratic = 2.0 * (a + b - 2.0 * mid);
    let linear = b - a - quadratic;
    let t = if quadratic.abs() > 1e-12 {
        (-linear / (2.0 * quadratic)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let max = a.abs().max(b.abs()).max(curve.slope_at(s.length * t).abs());
    if max > 0.200001 {
        return Err(format!(
            "Ramp too steep ({:.1}%); extend the road or reduce height difference",
            max * 100.0
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_length_and_grade_define_endpoint_height() {
        let s=MapSpline {length:30.0,pos:[100.0,100.0,7.0],..Default::default()};
        let r=reshape(&s,[5.0,25.0,4.0,12.0]).unwrap();
        let c=crate::SplineCurve::from_map(&r,glam::DVec2::ZERO);
        assert!((c.end_point().z-12.2).abs()<1e-9);assert_eq!(r.length,5.0);assert_eq!(r.radius,25.0);
        assert!((c.slope_at(2.5)-0.04).abs()<1e-9);
    }
    #[test]
    fn plane_intersection_works_above_and_below_ground() {
        assert_eq!(
            plane_hit(DVec3::new(1.0, 2.0, 10.0), -DVec3::Z, -5.0),
            Some(DVec3::new(1.0, 2.0, -5.0))
        );
        assert_eq!(
            plane_hit(DVec3::new(1.0, 2.0, -10.0), DVec3::Z, -5.0),
            Some(DVec3::new(1.0, 2.0, -5.0))
        );
        assert!(plane_hit(DVec3::ZERO, DVec3::X, -5.0).is_none());
        assert!(plane_hit(DVec3::ZERO, DVec3::Z, -5.0).is_none());
    }
    #[test]
    fn profile_preserves_xy_and_ends_exactly_with_smooth_ramps() {
        let s = omsi_map::MapSpline {
            pos: [1.0, 2.0, 0.0],
            length: 100.0,
            terrain_align: Some(1.0),
            terrain_align_flag: true,
            ..Default::default()
        };
        let p = height_profile(&s, [0.0, 10.0, 0.0, 0.0, 0.0]).unwrap();
        let c = crate::SplineCurve::from_map(&p, glam::DVec2::ZERO);
        assert!((c.end_point().z - 10.0).abs() < 1e-8);
        assert!(c.slope_at(0.0).abs() < 1e-8);
        assert!(c.slope_at(100.0).abs() < 1e-8);
        assert_eq!(p.pos[..2], s.pos[..2]);
        assert!(p.terrain_align.is_none() && !p.terrain_align_flag);
        assert!(height_profile(&s, [0.0, 20.0, 0.0, 0.0, 0.0]).is_err());
        assert!(height_profile(&s, [f64::NAN, 0.0, 0.0, 0.0, 0.0]).is_err());
    }
}

pub fn between(file: &str, id: i64, tile: (i32, i32), start: DVec3, end: DVec3, offset: f64) -> MapSpline {
    let d = end - start;
    let length = d.truncate().length();
    let grade = d.z / length.max(0.0001) * 100.0;
    let o = DVec2::new(tile.0 as f64, tile.1 as f64) * omsi_map::tile_size();
    MapSpline { file: file.into(), id, pos: [start.x - o.x, start.y - o.y, start.z],
        heading: d.x.atan2(d.y).to_degrees().rem_euclid(360.0), length,
        grad_start: grade, grad_end: grade, delta_h: Some(d.z), is_h: true,
        tex_offset: offset, map_chain_offset: Some(offset), ..Default::default() }
}

pub fn valid_spline(s: &MapSpline) -> bool {
    if s.profile_transitions.iter().flatten().any(|t| !t.valid()) { return false; }
    if !s.length.is_finite() || s.length <= 0.0 || !s.heading.is_finite()
        || !s.pos.iter().all(|n| n.is_finite()) || !s.radius.is_finite()
        || !s.grad_start.is_finite() || !s.grad_end.is_finite()
        || s.delta_h.is_some_and(|h| !h.is_finite()) { return false; }
    [s.cant_start, s.cant_end, s.skew_start, s.skew_end, s.tex_offset].iter().all(|n| n.is_finite())
}

pub fn inside_tile(s: &MapSpline) -> bool {
    if !valid_spline(s) { return false; }
    let c = SplineCurve::from_map(s, DVec2::ZERO);
    let size = omsi_map::tile_size();
    (0..=128).all(|i| { let p = c.point_at(s.length * i as f64 / 128.0); p.x >= -1e-6 && p.y >= -1e-6 && p.x <= size + 1e-6 && p.y <= size + 1e-6 })
}


pub fn sliced_transitions(s: &MapSpline, from: f64, length: f64) -> [Option<omsi_map::ProfileTransition>; 2] {
    s.profile_transitions.map(|t| t.and_then(|mut t| {
        t.station -= from;
        (t.station + t.span > 0.0 && t.station - t.span < length).then_some(t)
    }))
}

pub fn curve_part(s: &MapSpline, from: f64, length: f64) -> MapSpline {
    let c = SplineCurve::from_map(s, DVec2::ZERO);
    let p = c.point_at(from);
    let mut part = s.clone();
    part.profile_transitions = sliced_transitions(s, from, length);
    part.pos = [p.x, p.y, p.z];
    part.heading = c.heading_at(from);
    part.length = length;
    part.grad_start = c.slope_at(from) * 100.0;
    part.grad_end = c.slope_at(from + length) * 100.0;
    part.delta_h = Some(c.height_at(from + length) - p.z);
    part.is_h = true;
    part.cant_start = c.cant_at(from);
    part.cant_end = c.cant_at(from + length);
    part.skew_start = s.skew_start + (s.skew_end - s.skew_start) * from / s.length;
    part.skew_end = s.skew_start + (s.skew_end - s.skew_start) * (from + length) / s.length;
    part.tex_offset = s.tex_offset + from;
    part.map_chain_offset = Some(part.tex_offset);
    part
}


/// Exact tile boundary stations: split into monotone quadrants before root finding.
/// This also finds curves which leave and re-enter the same tile.
pub fn tile_sections(owner:(i32,i32), s:&MapSpline)->Vec<((i32,i32),MapSpline)> {
    let size=omsi_map::tile_size();
    let origin=DVec2::new(owner.0 as f64,owner.1 as f64)*size;
    let c=SplineCurve::from_map(s,origin);
    let mut monotone=vec![0.0,s.length];
    if s.radius!=0.0 {
        let a=c.heading_at(0.0);let b=c.heading_at(s.length);
        for n in (a.min(b)/90.0).floor() as i64..=(a.max(b)/90.0).ceil() as i64 {
            let u=(n as f64*90.0-a).to_radians()*s.radius;
            if u>0.0 && u<s.length {monotone.push(u);}
        }
    }
    monotone.sort_by(f64::total_cmp);
    let mut cuts=vec![0.0,s.length];
    for interval in monotone.windows(2) {for axis in 0..2 {
        let a=c.point_at(interval[0])[axis];let b=c.point_at(interval[1])[axis];
        if (a-b).abs()<1e-10 {continue;}
        for grid in (a.min(b)/size).ceil() as i64..=(a.max(b)/size).floor() as i64 {
            let boundary=grid as f64*size;let(mut lo,mut hi)=(interval[0],interval[1]);
            for _ in 0..55 {let mid=(lo+hi)*0.5;if (c.point_at(mid)[axis]<boundary)==(a<b){lo=mid;}else{hi=mid;}}
            cuts.push((lo+hi)*0.5);
        }
    }}
    cuts.sort_by(f64::total_cmp);cuts.dedup_by(|a,b|(*a-*b).abs()<1e-8);
    cuts.windows(2).map(|pair| {
        let mid=c.point_at((pair[0]+pair[1])*0.5);
        let tile=((mid.x/size).floor() as i32,(mid.y/size).floor() as i32);
        let mut part=curve_part(s,pair[0],pair[1]-pair[0]);
        part.pos[0]+=(owner.0-tile.0) as f64*size;part.pos[1]+=(owner.1-tile.1) as f64*size;
        (tile,part)
    }).collect()
}

#[cfg(test)]
mod tile_tests {
    use super::*;
    #[test]
    fn tile_cuts_preserve_curves_height_and_texture_in_all_directions() {
        for radius in [0.0,20.0,-20.0,0.5,-0.5,100000.0] {for heading in [0.0,45.0,90.0,180.0,270.0] {
            let s=reshape(&MapSpline{length:5.0,pos:[295.0,295.0,12.0],heading,tex_offset:3.0,..Default::default()},[150.0,radius,4.0,12.0]).unwrap();
            let owner=(-2,-3);let origin=DVec2::new(-600.0,-900.0);let whole=SplineCurve::from_map(&s,origin);
            let parts=tile_sections(owner,&s);let mut station=0.0;
            for (tile,part) in parts {
                let c=SplineCurve::from_map(&part,DVec2::new(tile.0 as f64,tile.1 as f64)*300.0);
                assert!((part.tex_offset-3.0-station).abs()<1e-8);
                for i in 0..=20 {let u=part.length*i as f64/20.0;assert!(c.point_at(u).distance(whole.point_at(station+u))<1e-6);}
                for i in 1..20 {let p=c.point_at(part.length*i as f64/20.0);assert_eq!(((p.x/300.0).floor() as i32,(p.y/300.0).floor() as i32),tile);}
                station+=part.length;
            }
            assert!((station-150.0).abs()<1e-8);
        }}
    }
}

/// A signed total turn, rather than a radius in metres.
pub fn turn_degrees(v:[f64;4])->f64 {if v[1]==0.0 {0.0}else{(v[0]/v[1]).to_degrees()}}
pub fn edit_dimension(mut v:[f64;4],field:usize,n:f64)->Result<[f64;4],String> {
    if field==4 {
        if !n.is_finite() {return Err("Enter a valid number".into());}
        v[1]=if n==0.0 {0.0}else{v[0]/n.to_radians()};
    } else {v[field]=n;}
    validate(v)?;Ok(v)
}
#[cfg(test)]
mod angle_tests {
    use super::*;
    #[test]
    fn degree_input_produces_a_real_quarter_circle() {
        for angle in [-90.0,90.0] {
            let v=edit_dimension([20.0,0.0,0.0,12.0],4,angle).unwrap();
            assert!((turn_degrees(v)-angle).abs()<1e-10);
            let s=reshape(&MapSpline{length:20.0,..Default::default()},v).unwrap();
            let c=SplineCurve::from_map(&s,DVec2::ZERO);
            assert!((c.heading_at(20.0)-angle).abs()<1e-10);
            assert!((c.end_point().x.abs()-v[1].abs()).abs()<1e-10);
            assert!((c.end_point().y-v[1].abs()).abs()<1e-10);
        }
        assert_eq!(edit_dimension([20.0,30.0,0.0,0.0],4,0.0).unwrap()[1],0.0);
        assert!(edit_dimension([20.0,0.0,0.0,0.0],4,f64::NAN).is_err());
    }
}

/// Exact grid crossings of a straight line, including negative tile coordinates.
pub fn straight_tile_cuts(start: DVec3, end: DVec3) -> Vec<f64> {
    let size = omsi_map::tile_size();
    let mut cuts = vec![0.0, 1.0];
    for (a, b) in [(start.x, end.x), (start.y, end.y)] {
        if (b - a).abs() < 1e-9 { continue; }
        let lo = (a.min(b) / size).floor() as i32;
        let hi = (a.max(b) / size).floor() as i32;
        for n in lo + 1..=hi {
            let t = (n as f64 * size - a) / (b - a);
            if t > 1e-9 && t < 1.0 - 1e-9 { cuts.push(t); }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    cuts
}

pub fn nearest_station(c: &SplineCurve, at: DVec3) -> f64 {
    let count = (c.length / 2.0).ceil().clamp(1.0, 4096.0) as usize;
    let step = c.length / count as f64;
    let distance = |d| (c.point_at(d).truncate() - at.truncate()).length_squared();
    let mut best = 0.0;
    for i in 1..=count {
        let d = i as f64 * step;
        if distance(d) < distance(best) { best = d; }
    }
    let (mut lo, mut hi) = ((best - step).max(0.0), (best + step).min(c.length));
    for _ in 0..32 {
        let a = lo + (hi - lo) / 3.0;
        let b = hi - (hi - lo) / 3.0;
        if distance(a) < distance(b) { hi = b; } else { lo = a; }
    }
    let middle = (lo + hi) * 0.5;
    [0.0, middle, c.length].into_iter().min_by(|a, b| distance(*a).total_cmp(&distance(*b))).unwrap_or(0.0)
}


pub fn profile_edges(s: &omsi_scenery::Spline) -> ((f64, f64), (f64, f64)) {
    let mut left = (f64::INFINITY, 0.0);
    let mut right = (f64::NEG_INFINITY, 0.0);
    for p in s.profiles.iter().flat_map(|p| &p.points) {
        if (p.x as f64) < left.0 { left = (p.x as f64, p.z as f64); }
        if (p.x as f64) > right.0 { right = (p.x as f64, p.z as f64); }
    }
    if !left.0.is_finite() {
        for p in &s.paths {
            let lo=p.start[0] as f64-p.width as f64*0.5;let hi=p.start[0] as f64+p.width as f64*0.5;
            if lo<left.0{left=(lo,p.start[2] as f64);}if hi>right.0{right=(hi,p.start[2] as f64);}
        }
        if !left.0.is_finite(){return ((0.0,0.0),(0.0,0.0));}
    }
    (left, right)
}

// The renderer and handles must share one geometry path (including skew and mirror).
pub fn physical_edges(def: &omsi_scenery::Spline, mirror: bool) -> [(f64, f64); 2] {
    let (left, right) = profile_edges(def);
    if mirror { [(-right.0, right.1), (-left.0, left.1)] } else { [left, right] }
}

/// Keep profiles and materials intact; a width taper may only join compatible
/// cross-sections. Never pretend that a sidewalk, lane merge or curb was invented.
pub fn compatible_sections(a: &omsi_scenery::Spline, am: bool, b: &omsi_scenery::Spline, bm: bool, reverse: bool) -> bool {
    let ae = physical_edges(a, am);
    let be = physical_edges(b, bm);
    let aw = ae[1].0 - ae[0].0;
    let bw = be[1].0 - be[0].0;
    if aw <= 0.001 || bw <= 0.001 { return false; }
    // Compare every surface vertex with the target surface at its normalised width.
    // The two directions detect extra target curbs/borders too.
    let check = |from: &omsi_scenery::Spline, fm: bool, to: &omsi_scenery::Spline, tm: bool,
                 fe: [(f64, f64); 2], te: [(f64, f64); 2]| {
        from.profiles.iter().filter(|p| p.points.len() >= 2).flat_map(|p| &p.points).all(|p| {
            let x = p.x as f64 * if fm { -1.0 } else { 1.0 };
            let t = (x - fe[0].0) / (fe[1].0 - fe[0].0);
            let u = if reverse { 1.0 - t } else { t };
            let tx = te[0].0 + u * (te[1].0 - te[0].0);
            let residual = p.z as f64 - (fe[0].1 * (1.0 - t) + fe[1].1 * t);
            to.profiles.iter().flat_map(|p| p.points.windows(2)).any(|pair| {
                let x0 = pair[0].x as f64 * if tm { -1.0 } else { 1.0 };
                let x1 = pair[1].x as f64 * if tm { -1.0 } else { 1.0 };
                if tx < x0.min(x1) - 1e-4 || tx > x0.max(x1) + 1e-4 { return false; }
                let target_base = te[0].1 * (1.0 - u) + te[1].1 * u;
                if (x1 - x0).abs() < 1e-6 {
                    let z = residual + target_base;
                    return z >= (pair[0].z.min(pair[1].z) as f64) - 0.005
                        && z <= (pair[0].z.max(pair[1].z) as f64) + 0.005;
                }
                let f = ((tx - x0) / (x1 - x0)).clamp(0.0, 1.0);
                let z = pair[0].z as f64 * (1.0 - f) + pair[1].z as f64 * f;
                (z - target_base - residual).abs() <= 0.005
            })
        })
    };
    check(a, am, b, bm, ae, be) && check(b, bm, a, am, be, ae)
}

pub fn tile_origin(tile: (i32, i32)) -> DVec2 {
    DVec2::new(tile.0 as f64, tile.1 as f64) * omsi_map::tile_size()
}

pub fn map_curve(tile: (i32, i32), s: &MapSpline) -> SplineCurve {
    SplineCurve::from_map(s, tile_origin(tile))
}

/// Monotone Hermite slopes prevent height overshoot between terrain samples.
pub fn terrain_parts(s: &MapSpline, heights: &[f64]) -> Vec<MapSpline> {
    let count = heights.len() - 1;
    let step = s.length / count as f64;
    let slopes: Vec<_> = heights.windows(2).map(|h| (h[1] - h[0]) / step).collect();
    let mut tangent = vec![slopes[0]];
    for pair in slopes.windows(2) {
        tangent.push(if pair[0] * pair[1] <= 0.0 { 0.0 } else { 2.0 * pair[0] * pair[1] / (pair[0] + pair[1]) });
    }
    tangent.push(*slopes.last().unwrap());
    let c = map_curve((0, 0), s);
    (0..count).map(|i| {
        let distance = i as f64 * step;
        let p = c.point_at(distance);
        let mut part = s.clone();
        part.profile_transitions = sliced_transitions(s, distance, step);
        part.pos = [p.x, p.y, heights[i]];
        part.heading = c.heading_at(distance);
        part.length = step;
        part.grad_start = tangent[i] * 100.0;
        part.grad_end = tangent[i + 1] * 100.0;
        part.delta_h = Some(heights[i + 1] - heights[i]);
        part.is_h = true;
        part.cant_start = s.cant_start + (s.cant_end - s.cant_start) * i as f64 / count as f64;
        part.cant_end = s.cant_start + (s.cant_end - s.cant_start) * (i + 1) as f64 / count as f64;
        part.skew_start = s.skew_start + (s.skew_end - s.skew_start) * i as f64 / count as f64;
        part.skew_end = s.skew_start + (s.skew_end - s.skew_start) * (i + 1) as f64 / count as f64;
        part.tex_offset = s.tex_offset + distance;
        part.map_chain_offset = Some(part.tex_offset);
        part
    }).collect()
}
