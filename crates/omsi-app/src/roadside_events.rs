//! Input routing for the road-object panel; camera flight stays available outside fields.
use crate::{app::App, roadside_objects::{self, Command, Window}};
use winit::keyboard::KeyCode;

impl App {
    pub(crate) fn editor_open_roadside(&mut self) {
        if self.net.lan.is_some() { self.service_msg = Some(("Object rows are available in single-player".into(), 5.0)); return; }
        self.editor_terrain_finish();
        let (Some(ed), Some(world)) = (self.menus.editor.as_mut(), self.world.as_ref()) else { return; };
        ed.splines.finish_drag(); ed.end_object_drag(); ed.splines.cancel_connection(); ed.splines.cancel_generation();
        ed.sidewalk_window=None;ed.catalog = None; ed.junction_window = None; ed.texture_target = None; ed.tile_window = None; ed.text_window = None;
        ed.terrain.active = false; ed.placing_asset = None; ed.object_stamp = None; ed.spline_mode = true;
        world.collect_editor_splines();
        let mut window = Window::new(ed.splines.selected); window.select(world, ed.splines.selected);
        ed.roadside_window = Some(window);
        self.menus.editor_drag = false; self.input.dragging = false; self.input.mouse_look = false; self.input.mmb_held = false;
        self.input.buttons_held = (false, false); self.input.keys.clear(); self.release_vehicle_keys();
        if let Some(window) = self.window.as_ref() { window.set_cursor_visible(true); }
        if let Some(ui) = self.ui.as_mut() { ui.end_editor_dock_drag(); }
    }
    pub(crate) fn editor_roadside_command(&mut self, command: Command) {
        let (Some(world), Some(ed)) = (self.world.clone(), self.menus.editor.as_mut()) else { return; };
        let Some(window) = ed.roadside_window.as_mut() else { return; };
        if matches!(command, Command::Close) {
            ed.roadside_window = None; self.input.keys.clear(); return;
        }
        if !window.commit() { return; }
        match command {
            Command::Catalog => {
                ed.catalog = Some(crate::asset_catalog::Catalog::with_map(world.root.clone(), crate::asset_catalog::Kind::Object, Some(world.map_dir.clone())));
                self.input.keys.clear(); self.input.mouse_look = false; self.input.mmb_held = false; self.input.buttons_held = (false, false); return;
            }
            Command::Edit(field) => { window.edit(field); self.input.keys.clear(); return; }
            Command::Adjust(field, amount) => {
                let value = window.value(field) + amount;
                if let Err(error) = window.set(field, value) { window.message = error; return; }
            }
            Command::Sides(sides) => window.settings.sides = sides,
            Command::Connected(connected) => { window.settings.connected = connected; window.settings.manual_gaps.clear(); window.gap_pick = None; }
            Command::Ground(ground) => window.settings.ground = ground,
            Command::PickGap => {
                window.gap_pick = Some(None); window.message = "Leave driveway clear: click its start, then its end on the road".into(); return;
            }
            Command::ClearGaps => { window.settings.manual_gaps.clear(); window.gap_pick = None; }
            Command::Apply => {
                let result = window.start.ok_or_else(|| "Select a road first".to_string())
                    .and_then(|start| roadside_objects::apply(&world, start, window.settings.clone(), window.file.clone()));
                window.message = match result { Ok(count) => format!("{count} objects placed · Ctrl+S to save · Ctrl+Z to undo"), Err(error) => error };
                window.refresh(&world); self.editor_reload_splines(); return;
            }
            Command::Undo => {
                window.message = match roadside_objects::undo(&world) { Ok(()) => "Object row undone · Ctrl+S to save".into(), Err(error) => error };
                window.refresh(&world); self.editor_reload_splines(); return;
            }
            Command::Remove => {
                let result = window.start.ok_or_else(|| "Select a road first".to_string()).and_then(|start| roadside_objects::remove(&world, start));
                window.message = match result { Ok(()) => "Own row removed · Ctrl+Z restores it · Ctrl+S to save".into(), Err(error) => error };
                window.refresh(&world); self.editor_reload_splines(); return;
            }
            Command::Save => { self.editor_action(crate::editor::Action::Save); return; }
            Command::Preview => window.message = "Blue = planned posts · Place applies the preview".into(),
            Command::Close => {}
        }
        window.refresh(&world);
    }
    pub(crate) fn editor_roadside_input(&mut self, code: Option<KeyCode>, text: Option<&str>, pressed: bool, repeat: bool) -> bool {
        if self.menus.game_menu.is_some() || !self.menus.editor.as_ref().is_some_and(|ed| ed.roadside_window.is_some() && ed.catalog.is_none()) { return false; }
        let editing = self.menus.editor.as_ref().unwrap().roadside_window.as_ref().unwrap().input.is_some();
        if repeat && !editing && matches!(code, Some(KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::KeyZ | KeyCode::KeyS | KeyCode::Escape)) { return true; }
        if let Some(key) = code {
            let modifier = matches!(key, KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::ShiftLeft | KeyCode::ShiftRight);
            let camera = matches!(key, KeyCode::KeyW | KeyCode::KeyA | KeyCode::KeyS | KeyCode::KeyD | KeyCode::KeyQ | KeyCode::KeyE
                | KeyCode::Space | KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown);
            if modifier || (camera && !editing && !self.input.keys.contains(&KeyCode::ControlLeft) && !self.input.keys.contains(&KeyCode::ControlRight)) {
                if pressed { self.input.keys.insert(key); } else { self.input.keys.remove(&key); }
                return true;
            }
            self.input.keys.remove(&key);
        }
        if !pressed { return true; }
        let ctrl = self.input.keys.contains(&KeyCode::ControlLeft) || self.input.keys.contains(&KeyCode::ControlRight);
        if !editing {
            let command = match code { Some(KeyCode::Escape) => Some(Command::Close),
                Some(KeyCode::Enter | KeyCode::NumpadEnter) => Some(Command::Apply),
                Some(KeyCode::KeyZ) if ctrl => Some(Command::Undo), Some(KeyCode::KeyS) if ctrl => Some(Command::Save), _ => None };
            if let Some(command) = command { self.editor_roadside_command(command); }
            return true;
        }
        let window = self.menus.editor.as_mut().unwrap().roadside_window.as_mut().unwrap();
        match code {
            Some(KeyCode::Escape) => { window.input = None; self.editor_roadside_command(Command::Preview); }
            Some(KeyCode::Enter | KeyCode::NumpadEnter) => { self.editor_roadside_command(Command::Preview); }
            Some(KeyCode::KeyA) if ctrl => { window.input.as_mut().unwrap().replace = true; }
            Some(KeyCode::Backspace) => {
                let input = window.input.as_mut().unwrap(); if input.replace { input.text.clear(); } else { input.text.pop(); } input.replace = false;
            }
            Some(KeyCode::Delete) => { let input = window.input.as_mut().unwrap(); input.text.clear(); input.replace = false; }
            _ if !ctrl => { if let Some(text) = text {
                let typed: String = text.chars().filter(|c| c.is_ascii_digit() || matches!(*c, '-' | '+' | ',' | '.')).collect();
                if !typed.is_empty() { let input = window.input.as_mut().unwrap(); if input.replace { input.text.clear(); input.replace = false; }
                    if input.text.len() + typed.len() <= 24 { input.text.push_str(&typed); } }
            } }
            _ => {}
        }
        true
    }
    pub(crate) fn editor_roadside_mouse(&mut self, pressed: bool) -> bool {
        if !self.menus.editor.as_ref().is_some_and(|ed| ed.roadside_window.is_some() && ed.catalog.is_none()) { return false; }
        if !pressed { return true; }
        if self.ui.as_mut().is_some_and(|ui| ui.editor_hud_press(self.input.cursor)) { return true; }
        let panel = self.menus.editor.as_ref().unwrap().roadside_window.as_ref().unwrap();
        if let Some(command) = panel.hit(self.input.cursor) { self.editor_roadside_command(command); return true; }
        if panel.contains(self.input.cursor) { return true; }
        let (Some(cam), Some(surface), Some(world)) = (self.camera.as_ref(), self.gfx.surface.as_ref(), self.world.clone()) else { return true; };
        let (origin, direction) = self.world_cursor_ray(cam, (surface.config.width, surface.config.height));
        let ed = self.menus.editor.as_mut().unwrap(); let window = ed.roadside_window.as_mut().unwrap();
        if !window.commit() { return true; }
        if let Some(first) = window.gap_pick {
            let hit = crate::placing::ground_hit(&world, origin, direction.as_dvec3(), 400.0);
            let result = window.start.zip(hit).ok_or_else(|| "Click the selected road".to_string())
                .and_then(|(start, hit)| roadside_objects::gap_station(&world, start, &window.settings, hit));
            match result {
                Ok(station) => if let Some(first) = first {
                    let (a, b) = (first.min(station), first.max(station));
                    if b - a < 0.5 { window.message = "The two boundaries must be at least 0.5 m apart".into(); }
                    else { window.settings.manual_gaps.push([a, b]); window.gap_pick = None;
                        window.message = format!("Area from {a:.1} to {b:.1} m stays clear"); window.refresh(&world); }
                } else { window.gap_pick = Some(Some(station)); window.message = format!("Start at {station:.1} m · Now click the end of the driveway"); },
                Err(error) => window.message = error,
            }
            return true;
        }
        ed.splines.pick(&world, origin, direction);
        let window = ed.roadside_window.as_mut().unwrap();
        window.select(&world, ed.splines.selected);
        self.menus.editor_drag = false; true
    }
}
