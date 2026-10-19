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

