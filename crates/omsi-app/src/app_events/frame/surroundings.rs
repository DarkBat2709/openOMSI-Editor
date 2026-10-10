//! The clock, the weather, the lights, the rain and the scenery's scripts in the window's
//! frame.

use super::*;

impl App {
    /// The METAR sync, the clock and the weather; the street lamps and the lit windows by the
    /// daylight, which the bus's scripts are told. The frame's daylight.
    pub(super) fn frame_weather(&mut self, dt: f32) -> omsi_sim::Daylight {
        // (the METAR sync: the report's weather, in real time)
        self.tick_metar(dt);
        if !self.paused {
            // (the time speed: the settings', or the session's in LAN play)
            let speed = self.time_speed();
            self.clock.advance(dt * speed as f32);
            // (the real-time sync: the device's date and time, whatever the speed was)
            self.sync_real_time();
            if let Some(t) = self.session.traffic.as_mut() {
                t.time_scale = speed;
            }
            self.tick_weather(dt * speed as f32);
        } else if self.session.weather_blend.is_some() {
            // (a preset picked in the paused menu: the change goes over in real time)
            self.tick_weather(dt);
        }
        let daylight = omsi_sim::Daylight::compute(&self.clock, self.session.envir.as_ref());
        let lamps = self.session.lamps_on != Some(daylight.lamps_on);
        self.session.lamps_on = Some(daylight.lamps_on);
        // the lit windows of the houses by their [NightMapMode] timetable (once a
        // second: tiles come and go, and the hours pass)
        let night_modes = self.perf.total_frames % 60 == 0;
        if let (Some(w), Some(r), Some(scene)) = (
            self.world.as_ref(),
            self.renderer.as_ref(),
            self.scene.as_mut(),
        ) {
            steps::world_lamps(w, r, scene, &self.clock, &daylight, lamps, night_modes);
        }
        if night_modes {
            self.follow_date();
        }
        if let Some(p) = self.player.as_mut() {
            steps::tell_surroundings(p, self.world.as_deref(), &daylight, self.session.weather.as_ref(), self.session.wetness);
        }
        daylight
    }

    /// The lights (the lamps' cones, the light maps, every light that shines), the rain and
    /// the snow, the cabin air, the tyres' spray and the sounds of the street.
    pub(super) fn frame_lights(&mut self, dt: f32, daylight: omsi_sim::Daylight) {
        let _diagnostic = crate::editor_diagnostics::Span::new("lights_and_path_overlay", "");
        let __t = Instant::now();
        // the lamps' cones in fog and falling rain or snow, and new light pictures
        if let Some(wt) = &self.session.weather {
            lights::set_cone_strength(wt.fog.0, precip_of(wt).1, daylight.night);
        }
        if let Some(r) = self.renderer.as_mut() {
            lights::upload_corona_textures(r);
        }
        // the tile light maps around the camera, for the roads' night light
        let __ta = Instant::now();
        if let (Some(w), Some(r), Some(cam)) = (self.world.as_ref(), self.renderer.as_ref(), self.camera.as_ref()) {
            w.update_light_map_atlas(r, cam.position);
        }
        *self.perf.profile.entry("lights.atlas").or_default() += __ta.elapsed().as_secs_f64();
        if let (Some(w), Some(scene), Some(cam)) = (
            self.world.as_ref(),
            self.scene.as_mut(),
            self.camera.as_ref(),
        ) {
            let vehicles = steps::light_vehicles(self.player.as_ref(), self.session.traffic.as_ref(), &self.net.remotes);
            let __tc = Instant::now();
                    lights::collect(w, scene, &daylight, cam.position, &vehicles);
                    *self.perf.profile.entry("lights.collect").or_default() += __tc.elapsed().as_secs_f64();
                    // Audit markers are independent of the current selection and editor mode.
                    if let Some(a)=self.menus.editor.as_ref().filter(|_|self.menus.game_menu.is_none()).and_then(|ed|ed.audit.as_ref()).filter(|a|a.markers&&!a.busy()) {
                        for e in a.report.nearby(cam.position,a.show_good) {
                            scene.coronas.push(omsi_render::Corona{position:e.at+glam::DVec3::Z*0.7,size:0.65,color:e.status.color(),brightness:2.0,..Default::default()});
                        }
                    }
                    // the object editor's pick: a magenta glow over it
                    if let Some(ed) = self.menus.editor.as_ref().filter(|ed| ed.spline_mode && !ed.terrain.active) {
                        let markers = if ed.splines.connection_active() { ed.splines.connection_markers(w) }
                            else { ed.splines.markers(w).into_iter().map(|p| (p, [1.0, 0.1, 0.9], 0.35)).collect() };
                        for (p, color, size) in markers {
                            scene.coronas.push(omsi_render::Corona {
                                position: p + glam::DVec3::Z * 0.4,
                                size,
                                color,
                                brightness: 2.0,
                                ..Default::default()
                            });
                        }
                    }
                    if let Some(ed)=self.menus.editor.as_mut().filter(|ed|ed.show_traffic_paths && self.menus.game_menu.is_none()) {
                        ed.traffic_overlay.refresh(w,cam.position);
                        let network=self.session.traffic.as_ref().map(|t|&t.net).or_else(||self.menus.navigator.as_ref().and_then(|n|n.map_net()));
                        let objects=network.into_iter().flat_map(|n|n.lanes.iter()).filter(|l|l.source!=1);
                        let mut targets=Vec::new();
                        for lane in ed.traffic_overlay.lanes.iter().chain(objects).filter(|l|l.kind==omsi_sim::traffic::LaneKind::Street
                            && !ed.traffic_window.as_ref().is_some_and(|w|w.replace&&w.error.is_none()&&l.key.is_some_and(|k|w.preview.replacements.iter().any(|(key,_,_)|*key==(k.tile,k.id))))) {
                            if let Some(key)=lane.key {for start in [true,false] {
                                let point=if start{lane.start()}else{lane.end()};
                                let heading=if start{lane.start_heading()}else{lane.end_heading()};
                                if point.distance(cam.position)<700.0 {targets.push(crate::traffic_editor::Target {point,direction:omsi_geometry::SplineCurve::dir(heading as f64).extend(0.0),start,key,reversed:lane.reversed});}
                            }}
                            let color=if lane.editor_bus_only {[1.0,0.6,0.0]}else{[0.0,0.7,1.0]};
                            for p in &lane.points {if p.distance(cam.position)<700.0 {scene.coronas.push(omsi_render::Corona {position:*p+glam::DVec3::Z*0.25,size:0.18,color,brightness:2.0,..Default::default()});}}
                            for station in (0..(lane.length()/15.0).ceil().min(100.0) as usize).map(|i|i as f32*15.0+2.0) {
                                let (p,h)=lane.at(station.min(lane.length()));if p.distance(cam.position)>700.0 {continue;}
                                let d=omsi_geometry::SplineCurve::dir(h as f64).extend(0.0);let side=glam::DVec3::new(d.y,-d.x,0.0);
                                for t in [0.0,0.25,0.5,0.75,1.0] {for sign in [-1.0,1.0] {
                                    scene.coronas.push(omsi_render::Corona {position:p-d*t+side*t*0.6*sign+glam::DVec3::Z*0.25,size:0.2,color,brightness:2.0,..Default::default()});
                                }}
                            }
                        }
                        if let Some(window)=ed.traffic_window.as_mut(){window.targets=targets;}
                    }
                    if let Some(panel)=self.menus.editor.as_ref().filter(|e|self.menus.game_menu.is_none()&&e.show_traffic_paths).and_then(|e|e.traffic_window.as_ref()){
                        if panel.free {if let Some(index)=panel.selected_node {if let Some(start)=panel.node_is_start(index) {
                            if let Some((_,point))=panel.preview.handles.iter().find(|(i,_)|*i==index) {
                                for target in panel.targets.iter().filter(|t|t.start!=start&&t.point.distance(*point)<50.0) {
                                    scene.coronas.push(omsi_render::Corona {position:target.point+glam::DVec3::Z*0.4,size:0.65,color:[0.9,0.1,1.0],brightness:1.0,..Default::default()});
                                }
                            }
                        }}}
                        if panel.free {for (index,p) in panel.preview.handles.iter().filter(|(i,_)|panel.settings.nodes[*i].0==panel.active_lane) {
                            scene.coronas.push(omsi_render::Corona {position:*p+glam::DVec3::Z*0.4,size:if panel.selected_node==Some(*index){0.85}else{0.55},color:if panel.selected_node==Some(*index){[1.0,0.8,0.0]}else{[0.1,1.0,0.2]},brightness:1.0,..Default::default()});
                        }}
                        for (p,bus) in panel.preview.markers.iter().filter(|(p,_)|p.distance(cam.position)<600.0){scene.coronas.push(omsi_render::Corona {position:*p+glam::DVec3::Z*0.15,size:0.22,color:if *bus{[1.0,0.6,0.0]}else{[0.0,0.7,1.0]},brightness:2.0,..Default::default()});}
                    }
                    if let Some(panel)=self.menus.editor.as_ref().filter(|ed|ed.catalog.is_none()&&self.menus.game_menu.is_none()).and_then(|e|e.sidewalk_window.as_ref()){
                        for p in panel.preview.markers.iter().filter(|p|p.distance(cam.position)<500.0){scene.coronas.push(omsi_render::Corona {position:*p+glam::DVec3::Z*0.15,size:0.18,color:[0.1,0.65,1.0],brightness:2.0,..Default::default()});}
                        for (i,p) in panel.preview.handles.iter().enumerate(){scene.coronas.push(omsi_render::Corona {position:*p+glam::DVec3::Z*0.4,size:0.85,color:if i==0{[0.1,1.0,0.2]}else{[1.0,0.6,0.1]},brightness:2.0,..Default::default()});}
                    }
                    if let Some(panel) = self.menus.editor.as_ref().filter(|ed| ed.catalog.is_none() && self.menus.game_menu.is_none()).and_then(|ed| ed.roadside_window.as_ref()) {
                        for point in panel.preview.points.iter().filter(|p| p.pos.distance(cam.position) < 500.0) {
                            for height in [0.15, 0.75, 1.35] {
                                scene.coronas.push(omsi_render::Corona { position: point.pos + glam::DVec3::Z * height,
                                    size: 0.2, color: [0.1, 0.55, 1.0], brightness: 2.0, ..Default::default() });
                            }
                        }
                    }
                    if let Some(ed)=self.menus.editor.as_ref().filter(|ed|ed.terrain.active && ed.tile_window.is_none() && self.menus.game_menu.is_none()) {
                        for p in ed.terrain.markers(w) {
                            scene.coronas.push(omsi_render::Corona {position:p,size:0.25,color:[0.15,0.65,1.0],brightness:2.0,..Default::default()});
                        }
                    }
                    if let Some(p) = self.menus.editor.as_ref().and_then(|ed| (!ed.terrain.active).then(||ed.selection_marker(w)).flatten()) {
                        scene.coronas.push(omsi_render::Corona {
                            position: p,
                            size: 0.6,
                            color: [1.0, 0.1, 0.9],
                            brightness: 2.0,
                            ..Default::default()
                        });
                    }
                    if let Some(wt) = &self.session.weather {
                let (kind, rate) = precip_of(wt);
                crate::rain::set_quality(&self.settings.rain_quality);
                self.session.rain.set(kind, rate);
                // [wind] direction (deg) speed (m/s)
                let wind = crate::rain::weather_wind(wt);
                let spray_wind = steps::spray_wind(wt);
                // every bus one may ride in keeps the weather out: the own, another
                // player's, a timetable bus - each part of it: an articulated bus's
                // rear section is a coupled part with its own [boundingbox] (#777)
                let boxed = crate::rain::vehicle_boxes;
                let mut buses: Vec<(glam::DVec3, f64, [f32; 6])> = self.player.as_ref().map(|p| boxed(&p.vehicle)).unwrap_or_default();
                buses.extend(self.net.remotes.remotes.values().flat_map(|rv| boxed(rv.vehicle())));
                if let Some(t) = self.session.traffic.as_ref() {
                    buses.extend(t.cars.iter().filter(|c| c.is_bus() && (c.vehicle.position - cam.position).length() < 40.0).flat_map(|c| boxed(&c.vehicle)));
                }
                let __tr = Instant::now();
                self.session.rain.tick(if self.paused { 0.0 } else { dt }, cam.position, wind, scene, &buses);
                // the player's bus's cabin air and the condensation on its glass
                if let Some(p) = self.player.as_ref() {
                    steps::cabin_air_step(&mut self.session.cabin_air, if self.paused { 0.0 } else { dt }, p, wt, self.session.humans.as_ref());
                }
                *self.perf.profile.entry("lights.rain").or_default() += __tr.elapsed().as_secs_f64();
                // what every vehicle's tyres throw up from the water on the road: the
                // puddles and the wet asphalt the renderer draws (the same wetness:
                // none under snow, OMSI_WETNESS as the picture takes it)
                let wetness = puddles::road_wetness(self.session.wetness, wt.snow);
                if (wetness > 0.0 || !self.session.spray.is_empty()) && !omsi_cfg::flags::OMSI_NO_SPRAY.is_set() {
                    let __ts = Instant::now();
                    steps::throw_spray(
                        &mut self.session.spray,
                        if self.paused { 0.0 } else { dt },
                        self.player.as_ref(),
                        self.session.traffic.as_ref(),
                        &self.net.remotes,
                        cam.position,
                        spray_wind,
                        w,
                        wetness,
                        crate::rain::quality(),
                    );
                    self.session.spray.sprites(cam.position, &mut scene.smoke);
                    *self.perf.profile.entry("lights.spray").or_default() += __ts.elapsed().as_secs_f64();
                }
                // the rain heard in the street and the footsteps on the pavement
                if let (Some(amb), Some(a)) = (self.sound.ambience.as_mut(), self.sound.audio.as_ref())
                {
                    let steps = self
                        .session.humans
                        .as_mut()
                        .map(|h| h.take_footfalls())
                        .unwrap_or_default();
                    // what the passengers say, where they stand
                    for line in self.session.humans.as_mut().map(|h| h.take_voice_lines()).unwrap_or_default() {
                        if let Some(clip) = a.load_clip(&line.path) {
                            a.play(
                                clip,
                                omsi_audio::mixer::VoiceParams {
                                    gain: 1.0,
                                    pitch: 1.0,
                                    looping: false,
                                    position: Some(line.position.as_vec3()),
                                    doppler: true,
                                    range: 3.0,
                                    lowpass_hz: 0.0,
                                    important: false,
                                },
                            );
                        }
                    }
                    let inside = self.cam.in_cab;
                    let __tm = Instant::now();
                    // openOMSI's ambience; with its recordings the street's rain is its own
                    let ours = if let Some(s) = self.sound.soundscape.as_mut() {
                        s.enabled = self.settings.ambient;
                        s.volume = self.settings.vol_ambient;
                        let people = self.session.humans.as_ref().map(|h| h.people.iter().filter(|p| (p.position - cam.position).length() < 40.0).count() as u32).unwrap_or(0);
                        let open = self.player.as_ref().and_then(|p| p.vehicle.var("Snd_OutsideVol")).unwrap_or(0.0);
                        s.update(
                            a,
                            crate::soundscape::Moment {
                                world: self.world.as_deref(),
                                weather: Some(wt),
                                clock: &self.clock,
                                sun: daylight.altitude_deg,
                                wetness,
                                ear: cam.position,
                                inside,
                                open,
                                people,
                                paused: self.paused,
                                dt,
                            },
                        );
                        s.active()
                    } else {
                        false
                    };
                    *self.perf.profile.entry("lights.soundscape").or_default() += __tm.elapsed().as_secs_f64();
                    amb.update(
                        a,
                        dt,
                        if ours { (0, 0.0) } else { (kind, rate) },
                        inside,
                        street_condition(wt, self.session.wetness),
                        cam.position,
                        &steps,
                    );
                    *self.perf.profile.entry("lights.ambience").or_default() += __tm.elapsed().as_secs_f64();
                    if let Some(every) = debug_sound_every() {
                        static LAST: std::sync::atomic::AtomicU32 =
                            std::sync::atomic::AtomicU32::new(u32::MAX);
                        let bucket = (self.clock.time / every as f64) as u32;
                        if LAST.swap(bucket, std::sync::atomic::Ordering::Relaxed) != bucket
                        {
                            log::info!("sound: environment - {} (precip {kind} {rate:.2}, StreetCond {:.2}, {} voices)", amb.last, street_condition(wt, self.session.wetness), a.voice_count());
                            if let Some(sc) = self.sound.soundscape.as_mut() {
                                let heard = sc.take_heard();
                                log::info!("sound: ambience - {}; heard {heard}", sc.last);
                            }
                        }
                    }
                }
            }
        }
        *self.perf.profile.entry("lights+rain").or_default() += __t.elapsed().as_secs_f64();
    }

    /// The departure boards, the map's route arrows and the scenery's scripts.
    pub(super) fn frame_scripted(&mut self, dt: f32, daylight: omsi_sim::Daylight) {
        let _diagnostic = crate::editor_diagnostics::Span::new("scenery_scripts", "");
        let __t = Instant::now();
        if let (Some(w), Some(r), Some(scene), Some(cam)) = (
            self.world.as_ref(),
            self.renderer.as_ref(),
            self.scene.as_mut(),
            self.camera.as_ref(),
        ) {
            let traffic = self.session.traffic.as_ref();
            let phase = |c: usize, li: usize| {
                traffic.map(|t| t.light_vars(c, li)).unwrap_or((omsi_sim::traffic::UNLINKED_PHASE as f32, 0.0))
            };
            let __tb = Instant::now();
            if let Some(p) = self.player.as_mut() {
                w.sync_html_departures(&mut p.vehicle.host);
            }
            steps::departure_boards(
                self.session.schedule.as_mut(),
                w,
                traffic,
                self.session.duty.as_ref(),
                self.player
                    .as_ref()
                    .and_then(|p| p.vehicle.host.hof.as_deref()),
                &self.clock,
            );
            *self.perf.profile.entry("scripted.boards").or_default() += __tb.elapsed().as_secs_f64();
            // the map's own route arrows, with OMSI 2's route arrows
            w.show_help_arrows(r, scene, self.settings.nav_arrows);
            w.update_scripted(
                r,
                scene,
                dt,
                cam.position,
                daylight.brightness,
                &phase,
                self.sound.audio.as_ref(),
                self.cam.in_cab,
            );
        }
        *self.perf.profile.entry("scripted").or_default() += __t.elapsed().as_secs_f64();
    }
}
