//! Per-instance, per-direction path changes stored as map rules. Shared SCO/SLI files
//! and path indices are untouched (timetables and traffic lights keep their identity).
use glam::DVec3;
use omsi_sim::traffic::{Lane, LaneKey};

pub const PREFIX: &str = "editor_lane_";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    pub key: LaneKey,
    pub source: u32,
    pub reversed: bool,
}
impl Id {
    pub fn of(l: &Lane) -> Option<Self> {
        Some(Self {
            key: l.key?,
            source: l.source,
            reversed: l.reversed,
        })
    }
}
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Change {
    pub deleted: bool,
    pub shape: [[f64; 3]; 2],
    pub ends: [[f64;3];2],
    pub floor: Option<[f64;65]>,
}
pub fn suffix(reverse: bool) -> &'static str {
    if reverse {
        "backward"
    } else {
        "forward"
    }
}
pub fn read(rules: &[omsi_map::MapRule], path: u16, reversed: bool) -> Change {
    let mut c = Change::default();let mut floor=[0.0;65];let mut found=[false;65];
    for r in rules.iter().filter(|r| {
        r.path_index == path as i32 && !r.kill && r.value.is_finite() && r.kind.starts_with(PREFIX)
    }) {
        if r.kind == format!("{PREFIX}deleted_{}", suffix(reversed)) {
            c.deleted = r.value != 0.0;
        }
        for i in 0..65 {if r.kind==format!("{PREFIX}floor_{i}_{}",suffix(reversed)){floor[i]=r.value;found[i]=true;}}
        for (i,name) in ["start","end"].iter().enumerate() {for (a,axis) in ["x","y","z"].iter().enumerate() {
            if r.kind==format!("{PREFIX}{name}_{axis}_{}",suffix(reversed)){c.ends[i][a]=r.value.clamp(-200.0,200.0);}
        }}
        for i in 0..2 {
            for (a, axis) in ["x", "y", "z"].iter().enumerate() {
                if r.kind == format!("{PREFIX}{}_{axis}_{}", i + 1, suffix(reversed)) {
                    c.shape[i][a] = r.value.clamp(-50.0, 50.0);
                }
            }
        }
    }
    if found.iter().all(|v|*v){c.floor=Some(floor);}
    c
}
// Height differences preserve original path clearance and deliberate vertical offsets.
pub fn surface_height(
    surface: &omsi_sim::traffic::EditorPathSurface,
    p: DVec3,
    seed: f64,
) -> Option<f64> {
    use omsi_sim::traffic::EditorPathSurface;
    match surface {
        EditorPathSurface::Spline(curve) => {
            let mut station = seed.clamp(0.0, curve.length);
            let mut x = 0.0;
            for _ in 0..16 {
                let center = curve.point_at(station);
                let forward = omsi_geometry::SplineCurve::dir(curve.heading_at(station));
                let side = glam::DVec2::new(forward.y, -forward.x);
                x = (p.truncate() - center.truncate()).dot(side);
                let q = curve.profile_point(station, x, 0.0);
                let next =
                    (station + (p.truncate() - q.truncate()).dot(forward)).clamp(0.0, curve.length);
                if (next - station).abs() < 1e-6 {
                    break;
                }
                station = next;
            }
            let h = curve.profile_point(station, x, 0.0).z;
            h.is_finite().then_some(h)
        }
        EditorPathSurface::Object {
            pos,
            transform,
            field,
        } => {
            let inv = transform.inverse();
            let local = inv.transform_point3((p - *pos).as_vec3());
            let vertical = inv.transform_vector3(glam::Vec3::Z);
            if vertical.z.abs() < 1e-5 {
                return None;
            }
            let on_plane = local - vertical * (local.z / vertical.z);
            let base = *pos + transform.transform_point3(on_plane).as_dvec3();
            let height = field
                .as_ref()
                .and_then(|f| crate::field_height(f, on_plane.x, on_plane.y))
                .unwrap_or(0.0);
            let h = base.z + height as f64;
            h.is_finite().then_some(h)
        }
    }
}
pub fn attach_roads(lanes: &mut [Lane], roads: &[omsi_geometry::SplineCurve], editable:bool) {
    for l in lanes {
        if !editable && l.editor_path_base.is_empty()&&!l.editor_additional_connection {continue;}
        let (start,length)=match l.editor_path_surface.as_ref() {
            Some(omsi_sim::traffic::EditorPathSurface::Spline(owner))=>(owner.start,owner.length),
            Some(omsi_sim::traffic::EditorPathSurface::Object {..})=>(l.start(),l.length() as f64),
            None=>continue,
        };
        l.editor_path_roads = Some(std::sync::Arc::new(
            roads
                .iter()
                .filter(|c| {
                    c.start.truncate().distance(start.truncate())
                        < c.length + length + 250.0
                })
                .copied()
                .collect(),
        ));
        if !l.editor_path_base.is_empty()||l.editor_additional_connection {
            apply_shape(l, l.editor_path_shape);
        }
    }
}
fn neighboring_height(roads: &[omsi_geometry::SplineCurve], p: DVec3) -> Option<f64> {
    roads
        .iter()
        .filter_map(|c| {
            if c.start.truncate().distance(p.truncate()) > c.length + 20.0 {
                return None;
            }
            let n = (c.length / 10.0).ceil().clamp(1.0, 128.0) as usize;
            let mut station = (0..=n)
                .map(|i| c.length * i as f64 / n as f64)
                .min_by(|a, b| {
                    c.point_at(*a)
                        .truncate()
                        .distance_squared(p.truncate())
                        .total_cmp(&c.point_at(*b).truncate().distance_squared(p.truncate()))
                })?;
            for _ in 0..16 {
                let q = c.point_at(station);
                let d = omsi_geometry::SplineCurve::dir(c.heading_at(station));
                let next = (station + (p.truncate() - q.truncate()).dot(d)).clamp(0.0, c.length);
                if (next - station).abs() < 1e-6 {
                    break;
                }
                station = next;
            }
            let center = c.point_at(station);
            let d = omsi_geometry::SplineCurve::dir(c.heading_at(station));
            let delta = p.truncate() - center.truncate();
            // End projection must be inside the road, not clamped to a remote endpoint.
            if delta.dot(d).abs() > 0.5 {
                return None;
            }
            let x = delta.dot(glam::DVec2::new(d.y, -d.x));
            if x.abs() > c.half_cant_width.max(5.5) + 1.0 {
                return None;
            }
            let height = surface_height(
                &omsi_sim::traffic::EditorPathSurface::Spline(*c),
                p,
                station,
            )?;
            Some((
                delta.length_squared() + (height - p.z).powi(2) * 0.05,
                height,
            ))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, h)| h)
}
pub fn apply_shape(lane: &mut Lane, shape: [[f64; 3]; 2]) {
    if lane.editor_path_base.is_empty() {
        lane.editor_path_base = lane.points.clone();
        lane.editor_path_headings = lane.headings.clone();
        lane.editor_path_curvature = lane.curvature.clone();
    }
    let base = &lane.editor_path_base;
    if base.len() < 2 {
        return;
    }
    let dense: Vec<DVec3> = if shape == [[0.0; 3]; 2] && lane.editor_path_ends==[[0.0;3];2] {
        base.clone()
    } else {
        let count = base.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>();
        let n = (count / 0.25).ceil().clamp((base.len() - 1) as f64, 4000.0) as usize;
        (0..=n)
            .map(|i| {
                let at = i as f64 / n as f64 * (base.len() - 1) as f64;
                let j = (at.floor() as usize).min(base.len() - 2);
                base[j].lerp(base[j + 1], at - j as f64)
            })
            .collect()
    };
    lane.points = dense.clone();
    let base = &dense;
    let last = base.len() - 1;
    for (i, p) in lane.points.iter_mut().enumerate() {
        let t = i as f64 / last as f64;
        // Smooth displacement with fixed endpoints AND fixed endpoint tangents.
        let envelope = 16.0 * t * t * (1.0 - t) * (1.0 - t);
        let u = ((t - 1.0 / 3.0) * 3.0).clamp(0.0, 1.0);
        let u = u * u * (3.0 - 2.0 * u);
        let v = DVec3::from_array(shape[0]) * (1.0 - u) + DVec3::from_array(shape[1]) * u;
        let d = (base[(i + 1).min(last)] - base[i.saturating_sub(1)])
            .truncate()
            .normalize_or_zero();
        let forward = d.extend(0.0);
        let side = DVec3::new(d.y, -d.x, 0.0);
        *p += envelope * (side * v.x + forward * v.y + DVec3::Z * v.z)
            +DVec3::from_array(lane.editor_path_ends[0])*(1.0-t)+DVec3::from_array(lane.editor_path_ends[1])*t;
        if i > 0 && i < last && (p.truncate() - base[i].truncate()).length_squared() > 1e-12 {
            if let Some(surface) = lane.editor_path_surface.as_ref() {
                let seed = match surface {
                    omsi_sim::traffic::EditorPathSurface::Spline(c) => {
                        c.length * if lane.reversed { 1.0 - t } else { t }
                    }
                    _ => 0.0,
                };
                if let (Some(before), Some(after)) = (
                    surface_height(surface, base[i], seed),
                    lane.editor_path_roads
                        .as_ref()
                        .and_then(|roads| neighboring_height(roads, *p))
                        .or_else(|| surface_height(surface, *p, seed)),
                ) {
                    p.z = base[i].z+envelope*v.z+after-before;
                }
            }
        }
    }
    if let Some(heights)=lane.editor_path_floor {
        let last=lane.points.len()-1;
        for (i,p) in lane.points.iter_mut().enumerate() {let at=i as f64/last as f64*64.0;let j=(at.floor() as usize).min(63);p.z=heights[j]*(1.0-(at-j as f64))+heights[j+1]*(at-j as f64);}
    }
    let measured =
        omsi_sim::traffic::LaneBuilder::polyline(lane.points.clone(), lane.kind, lane.width);
    lane.headings = measured.headings;
    if let (Some(a), Some(b)) = (
        lane.editor_path_headings.first(),
        lane.editor_path_headings.last(),
    ) {
        if lane.editor_path_ends==[[0.0;3];2] {lane.headings[0] = *a;
        let end = lane.headings.len() - 1;
        lane.headings[end] = *b;}
    }
    lane.curvature = measured.curvature;
    lane.dist = measured.dist;
    if shape == [[0.0; 3]; 2] && lane.editor_path_ends==[[0.0;3];2] {
        lane.headings = lane.editor_path_headings.clone();
        lane.curvature = lane.editor_path_curvature.clone();
    }
    if lane.editor_path_ends[0]!=[0.0;3] {lane.editor_previous=None;}
    if lane.editor_path_ends[1]!=[0.0;3] {lane.editor_next=None;}
    lane.editor_path_shape = shape;
}
pub fn from_rules(lane: &mut Lane, rules: &[omsi_map::MapRule]) -> bool {
    let Some(key) = lane.key else {
        return true;
    };
    let c = read(rules, key.path, lane.reversed);
    if c.deleted {
        return false;
    }
    lane.editor_path_ends=c.ends;lane.editor_path_floor=c.floor;
    if c.shape != [[0.0; 3]; 2] || c.ends!=[[0.0;3];2] || c.floor.is_some() {
        apply_shape(lane, c.shape);
    }
    true
}
