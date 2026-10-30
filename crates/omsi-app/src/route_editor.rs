//! Author a bus route on saved, directed traffic paths and store native OMSI TTData.
use crate::{
    app::App,
    editor::{self},
};
use glam::DVec3;
use omsi_sim::traffic::{LaneKind, Network};
use omsi_timetable::{
    BusStopEntry, Line, StnLink, StnLinkEntry, TimetableData, Tour, TourTrip, Trip, TripProfile,
};
use std::path::Path;
use winit::keyboard::KeyCode;
#[derive(Clone, Copy)]
pub enum Command {
    Field(usize),
    Stop(usize),
    Page(i32),
    Remove,
    Save,
    Close,
}
pub struct Window {
    pub fields: [String; 5],
    pub input: Option<(usize, bool)>,
    pub stops: Vec<BusStopEntry>,
    pub chosen: Vec<usize>,
    pub page: usize,
    pub net: Network,
    pub places: std::collections::HashMap<i64, DVec3>,
    pub tile_coords: Vec<(i32, i32)>,
    pub buttons: Vec<([f32; 4], Command)>,
    pub rect: Option<[f32; 4]>,
    pub message: String,
}
fn file_name(s: &str) -> bool {
    let base = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
    {
        return false;
    }
    !s.trim().is_empty()
        && s.len() <= 100
        && !s.ends_with(['.', ' '])
        && !s
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        && !s.starts_with('.')
}
fn departure(s: &str) -> Result<f32, String> {
    let p: Vec<_> = s.split(':').collect();
    if p.len() != 2 {
        return Err("Use HH:MM for departure".into());
    }
    let h = p[0].parse::<u32>().map_err(|_| "Use HH:MM for departure")?;
    let m = p[1].parse::<u32>().map_err(|_| "Use HH:MM for departure")?;
    if h >= 48 || m >= 60 {
        return Err("Departure must be between 00:00 and 47:59".into());
    }
    Ok((h * 60 + m) as f32)
}
impl Window {
    fn lane(&self, id: i64) -> Result<(usize, f32), String> {
        let p = *self
            .places
            .get(&id)
            .ok_or("Stop position not found; save and reload the map")?;
        let (l, s, d) = self
            .net
            .nearest_lane(p, LaneKind::Street)
            .ok_or("No driving path near stop")?;
        if d > 40.0 {
            return Err("Stop is more than 40 m from a driving path".into());
        }
        Ok((l, s))
    }
    pub fn link(&self, a: usize, b: usize) -> Result<StnLink, String> {
        let (from, to) = (self.stops[a].object_id, self.stops[b].object_id);
        if from == to {
            return Err("Choose a different next stop".into());
        }
        let ((start, start_s), (end, end_s)) = (self.lane(from)?, self.lane(to)?);
        if start == end && end_s <= start_s {
            return Err("Stops run against the direction of this driving path".into());
        }
        let path = self.net.shortest_path(start, end).ok_or_else(|| {
            format!(
                "No driving route: {} → {}",
                self.stops[a].name, self.stops[b].name
            )
        })?;
        let mut entries = Vec::new();
        let mut length = 0.0;
        for i in path {
            let l = &self.net.lanes[i];
            let k = l.key.ok_or("Driving path has no saved map ID")?;
            let tile = self
                .tile_coords
                .iter()
                .position(|t| *t == k.tile)
                .ok_or("Driving path tile not in map")?;
            length += l.length() as f64;
            entries.push(StnLinkEntry {
                values: [
                    k.id as f64,
                    k.path as f64,
                    tile as f64,
                    l.length() as f64,
                    0.0,
                    0.0,
                    0.0,
                ],
            });
        }
        Ok(StnLink {
            length,
            from_id: from,
            to_id: to,
            entries,
            ..Default::default()
        })
    }
    fn save(&self, folder: &Path, source: &Path, original: &Path) -> Result<(), String> {
        let [line, name, time, duration, group] = &self.fields;
        if !file_name(line) || !file_name(name) {
            return Err("Enter valid line and route names".into());
        }
        let dep = departure(time)?;
        let minutes = duration
            .replace(',', ".")
            .parse::<f32>()
            .map_err(|_| "Enter the trip duration in minutes")?;
        if !minutes.is_finite() || minutes <= 0.0 || minutes > 1440.0 {
            return Err("Trip duration must be between 0 and 1440 minutes".into());
        }
        if self.chosen.len() < 2 {
            return Err("A route needs at least two stops".into());
        }
        let links: Vec<_> = self
            .chosen
            .windows(2)
            .map(|w| self.link(w[0], w[1]))
            .collect::<Result<_, _>>()?;
        let target = editor::transit_save::ensure_ttdata(folder, source, original)?;
        let existing = TimetableData::load(folder);
        if existing
            .trips
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case(name))
        {
            return Err("This route name already exists; choose a new name".into());
        }
        let trip_path = target.join(format!("{name}.ttp"));
        if trip_path.exists() {
            return Err("This route file already exists".into());
        }
        let trip = Trip {
            line: line.clone(),
            terminus: self.stops[*self.chosen.last().unwrap()].name.clone(),
            stations: self
                .chosen
                .iter()
                .map(|i| self.stops[*i].object_id)
                .collect(),
            profiles: vec![TripProfile {
                name: "standard".into(),
                factor: minutes,
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut line_data = existing
            .lines
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(line))
            .cloned()
            .unwrap_or_else(|| Line {
                name: line.clone(),
                user_allowed: true,
                ..Default::default()
            });
        line_data.user_allowed = true;
        let number = (1..)
            .find(|n| !line_data.tours.iter().any(|t| t.number == n.to_string()))
            .unwrap();
        line_data.tours.push(Tour {
            number: number.to_string(),
            ai_group: group.clone(),
            trips: vec![TourTrip {
                trip: name.clone(),
                profile: 0,
                departure: dep,
            }],
            ..Default::default()
        });
        let line_path = target.join(
            line_data
                .path
                .file_name()
                .map(|p| p.to_owned())
                .unwrap_or_else(|| format!("{line}.ttl").into()),
        );
        let mut all_links = existing.stn_links;
        for link in links {
            if !all_links
                .iter()
                .any(|l| l.from_id == link.from_id && l.to_id == link.to_id)
            {
                all_links.push(link);
            }
        }
        let mut text = String::new();
        for l in all_links {
            text += &format!("[StnLink]\n{}\n{}\n{}\n", l.length, l.from_id, l.to_id);
            for n in l.params {
                text += &format!("{n}\n");
            }
            for e in l.entries {
                text += "[StnLink_entry]\n";
                for n in e.values {
                    text += &format!("{n}\n");
                }
            }
            text.push('\n');
        }
        // Files are prepared and validated before writing; the line is committed last, so a
        // failed intermediate write cannot expose a tour whose trip does not exist.
        editor::save_copy(&target.join("StnLinks.cfg"), &omsi_cfg::encode_text(&text))?;
        editor::save_copy(&trip_path, &omsi_cfg::encode_text(&trip.to_text()))?;
        if let Err(e) = editor::save_copy(&line_path, &omsi_cfg::encode_text(&line_data.to_text()))
        {
            // This trip was created by this attempt; leave an existing timetable untouched on failure.
            let _ = std::fs::remove_file(&trip_path);
            return Err(e);
        }
        omsi_cfg::content_changed();
        Ok(())
    }
}
impl App {
    pub(crate) fn editor_place_transit(&mut self, entry: bool) {
        let (Some(ed), Some(w)) = (self.menus.editor.as_mut(), self.world.as_ref()) else {
            return;
        };
        let file = if entry {
            "Sceneryobjects/Generic/entrypoint_bus.sco"
        } else {
            "Sceneryobjects/Generic/bus_stop.sco"
        };
        let path = omsi_cfg::resolve_path(&w.root, file);
        if !path.is_file() {
            self.service_msg = Some(("The original OMSI helper object is missing".into(), 6.0));
            return;
        }
        let mut stamp = editor::ObjectStamp::new(path);
        stamp.strings = if entry {
            vec!["Start".into()]
        } else {
            ["Haltestelle", "5", "0", "2", "30", "0", ""]
                .map(String::from)
                .to_vec()
        };
        ed.start_object(stamp);
        ed.repeat_objects = false;
        self.service_msg = Some((
            "Click to place · N/M direction · Edit text to name · Ctrl+S to save".into(),
            8.0,
        ));
    }
    pub(crate) fn editor_open_routes(&mut self) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        // The explicit save is left to the user: reading the network never silently commits edits.
        let nav = world.navigation_map();
        let mut net = Network {
            lanes: nav
                .lanes
                .into_iter()
                .filter(|l| l.kind == LaneKind::Street)
                .collect(),
            ..Default::default()
        };
        net.link(1.5);
        let data = TimetableData::load(world.global.dir());
        let mut stops = data.bus_stops;
        stops.sort_by(|a, b| a.name.cmp(&b.name).then(a.object_id.cmp(&b.object_id)));
        let mut seen = std::collections::HashSet::new();
        stops.retain(|s| seen.insert(s.object_id));
        let w=Window{fields:["52".into(),"52 Hinfahrt".into(),"08:00".into(),"20".into(),String::new()],input:None,stops,chosen:Vec::new(),page:0,net,places:nav.places.into_iter().collect(),tile_coords:world.global.raw_tiles.clone(),buttons:Vec::new(),rect:None,message:"Save and reload map after placing stops or changing roads. Choose stops in travel order.".into()};
        if let Some(ed) = self.menus.editor.as_mut() {
            self.input.keys.clear();
            ed.route_window = Some(w);
            ed.placing_asset = None;
            ed.object_stamp = None;
        }
    }
    fn editor_route_command(&mut self, c: Command) {
        let Some(mut w) = self
            .menus
            .editor
            .as_mut()
            .and_then(|e| e.route_window.take())
        else {
            return;
        };
        w.input = None;
        match c {
            Command::Close => {
                self.input.keys.clear();
                return;
            }
            Command::Field(i) => {
                w.input = Some((i, true));
                self.input.keys.clear();
            }
            Command::Page(d) => {
                w.page = (w.page as i32 + d).clamp(0, w.stops.len().saturating_sub(1) as i32 / 6)
                    as usize;
            }
            Command::Remove => {
                w.chosen.pop();
            }
            Command::Stop(i) => {
                if let Some(&last) = w.chosen.last() {
                    match w.link(last, i) {
                        Ok(_) => w.chosen.push(i),
                        Err(e) => w.message = e,
                    }
                } else {
                    w.chosen.push(i);
                }
            }
            Command::Save => {
                if let (Some(world), Some(content)) =
                    (self.world.as_ref(), omsi_launcher_lib::content_dir())
                {
                    let folder = content.join(
                        Path::new(&self.args.map)
                            .parent()
                            .unwrap_or(Path::new("maps")),
                    );
                    w.message = match w.save(&folder, world.global.dir(), &self.args.root) {
                        Ok(()) => "Route and line saved · Reload map to drive the new tour".into(),
                        Err(e) => e,
                    };
                } else {
                    w.message = "Content folder unavailable".into();
                }
            }
        }
        self.menus.editor.as_mut().unwrap().route_window = Some(w);
    }
    pub(crate) fn editor_route_mouse(&mut self, pressed: bool) -> bool {
        let Some(w) = self
            .menus
            .editor
            .as_ref()
            .and_then(|e| e.route_window.as_ref())
        else {
            return false;
        };
        let c = if pressed {
            w.buttons
                .iter()
                .find(|(r, _)| crate::audit_events::contains(*r, self.input.cursor))
                .map(|(_, c)| *c)
        } else {
            None
        };
        if let Some(c) = c {
            self.editor_route_command(c);
        }
        true
    }
    pub(crate) fn editor_route_input(
        &mut self,
        code: Option<KeyCode>,
        text: Option<&str>,
        pressed: bool,
    ) -> bool {
        let Some(w) = self
            .menus
            .editor
            .as_mut()
            .and_then(|e| e.route_window.as_mut())
        else {
            return false;
        };
        if let Some(k) = code {
            self.input.keys.remove(&k);
        }
        if !pressed {
            return true;
        }
        if code == Some(KeyCode::Escape) {
            self.editor_route_command(Command::Close);
            return true;
        }
        if code == Some(KeyCode::Tab) {
            let i = w.input.map_or(0, |(i, _)| (i + 1) % 5);
            w.input = Some((i, true));
            return true;
        }
        if code == Some(KeyCode::Enter) {
            w.input = None;
            return true;
        }
        if let Some((i, replace)) = w.input.as_mut() {
            let value = &mut w.fields[*i];
            if matches!(code, Some(KeyCode::Backspace | KeyCode::Delete)) {
                if *replace || code == Some(KeyCode::Delete) {
                    value.clear();
                } else {
                    value.pop();
                }
                *replace = false;
            } else if let Some(t) = text {
                let t: String = t.chars().filter(|c| !c.is_control()).collect();
                if !t.is_empty() {
                    if *replace {
                        value.clear();
                        *replace = false;
                    }
                    if value.len() + t.len() <= 100 {
                        value.push_str(&t);
                    }
                }
            }
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_and_departures_are_validated() {
        for s in ["../52", "", "a/b", "a\\b", "a\n"] {
            assert!(!file_name(s));
        }
        assert!(file_name("52 Hinfahrt"));
        assert_eq!(departure("25:10").unwrap(), 1510.0);
        for s in ["48:00", "12:60", "-1:00", "abc"] {
            assert!(departure(s).is_err());
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use omsi_sim::traffic::{LaneBuilder, LaneKey};
    pub(super) fn fixture() -> Window {
        let mut lane = LaneBuilder::polyline(
            vec![DVec3::ZERO, DVec3::new(0.0, 100.0, 0.0)],
            LaneKind::Street,
            3.0,
        );
        lane.key = Some(LaneKey {
            tile: (0, 0),
            id: 42,
            path: 0,
        });
        Window {
            fields: [
                "52".into(),
                "52 outward".into(),
                "08:15".into(),
                "12".into(),
                "Buses".into(),
            ],
            input: None,
            stops: vec![
                BusStopEntry {
                    name: "Start".into(),
                    object_id: 1,
                    ..Default::default()
                },
                BusStopEntry {
                    name: "End".into(),
                    object_id: 2,
                    ..Default::default()
                },
            ],
            chosen: vec![0, 1],
            page: 0,
            net: Network {
                lanes: vec![lane],
                ..Default::default()
            },
            places: [
                (1, DVec3::new(1.0, 10.0, 0.0)),
                (2, DVec3::new(1.0, 90.0, 0.0)),
            ]
            .into_iter()
            .collect(),
            tile_coords: vec![(0, 0)],
            buttons: vec![],
            rect: None,
            message: String::new(),
        }
    }
    #[test]
    fn direction_and_missing_paths_are_rejected() {
        let mut w = fixture();
        assert!(w.link(0, 1).is_ok());
        assert!(w.link(1, 0).is_err());
        w.net.lanes.clear();
        assert!(w.link(0, 1).is_err());
    }
    #[test]
    fn save_round_trip_preserves_existing_tours_and_rejects_overwrite() {
        let root = std::env::temp_dir().join(format!("route-editor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("original/maps/Test");
        let output = root.join("content/maps/Test");
        std::fs::create_dir_all(source.join("TTData")).unwrap();
        let old = Line {
            name: "52".into(),
            user_allowed: false,
            tours: vec![Tour {
                number: "1".into(),
                ai_group: "Old buses".into(),
                trips: vec![TourTrip {
                    trip: "old".into(),
                    profile: 0,
                    departure: 60.0,
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        std::fs::write(
            source.join("TTData/52.ttl"),
            omsi_cfg::encode_text(&old.to_text()),
        )
        .unwrap();
        let mut w = fixture();
        w.save(&output, &source, &root.join("original")).unwrap();
        let trip = Trip::load(&output.join("TTData/52 outward.ttp")).unwrap();
        assert_eq!(trip.stations, vec![1, 2]);
        assert_eq!(trip.profiles[0].factor, 12.0);
        let line = Line::load(&output.join("TTData/52.ttl")).unwrap();
        assert!(line.user_allowed);
        assert_eq!(line.tours.len(), 2);
        assert_eq!(line.tours[0].trips[0].trip, "old");
        assert_eq!(line.tours[1].trips[0].departure, 495.0);
        let before = std::fs::read(output.join("TTData/52.ttl")).unwrap();
        assert!(w.save(&output, &source, &root.join("original")).is_err());
        assert_eq!(before, std::fs::read(output.join("TTData/52.ttl")).unwrap());
        w.fields[1] = "52 second trip".into();
        w.save(&output, &source, &root.join("original")).unwrap();
        let line = Line::load(&output.join("TTData/52.ttl")).unwrap();
        assert_eq!(line.tours.len(), 3);
        assert_eq!(line.tours[1].trips[0].trip, "52 outward");
        assert_eq!(
            std::fs::read(source.join("TTData/52.ttl")).unwrap(),
            omsi_cfg::encode_text(&old.to_text())
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod visual_tests {
    #[test]
    #[ignore = "requires a GPU; writes editor interface review images to /tmp"]
    fn render_route_and_object_panels() {
        use omsi_render::{Camera, Lighting, RenderOptions, Renderer};
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let mut r = pollster::block_on(Renderer::new_with(
            &instance,
            None,
            Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            RenderOptions {
                msaa: 1,
                ray_tracing: false,
                ..Default::default()
            },
        ))
        .unwrap();
        let mut ui = crate::ui::Ui::new().unwrap();
        crate::ui_language("DEU");
        let mut ed = crate::editor::Editor::default();
        ed.route_window = Some(super::persistence_tests::fixture());
        let camera = Camera {
            position: glam::DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            fov_deg: 70.0,
            near: 0.1,
            far: 1000.0,
        };
        for (width, height) in [(1280, 900), (800, 600)] {
            let mut scene = r.new_scene();
            ui.draw_route_editor(
                &r,
                &mut scene,
                Some(&mut ed),
                1.0,
                [0.0, 0.0, width as f32, height as f32],
                (-1.0, -1.0),
            );
            for (rect, _) in &ed.route_window.as_ref().unwrap().buttons {
                assert!(
                    rect[0] >= 0.0
                        && rect[1] >= 0.0
                        && rect[2] <= width as f32
                        && rect[3] <= height as f32
                );
            }
            let bytes = r
                .render_to_image(&mut scene, width, height, &camera, &Lighting::default())
                .unwrap();
            image::save_buffer(
                format!("/tmp/editor-routes-{width}.png"),
                &bytes,
                width,
                height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        let mut scene = r.new_scene();
        ui.draw_editor_dock(
            &r,
            &mut scene,
            Some((false, true)),
            None,
            None,
            false,
            false,
            Some((false, false, false, false, false)),
            None,
            1.0,
            [0.0, 0.0, 1280.0, 900.0],
            (-1.0, -1.0),
        );
        let bytes = r
            .render_to_image(&mut scene, 1280, 900, &camera, &Lighting::default())
            .unwrap();
        image::save_buffer(
            "/tmp/editor-objects.png",
            &bytes,
            1280,
            900,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
