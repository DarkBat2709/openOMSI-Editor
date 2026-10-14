//! Explicit spline heights and an editor-only construction plane for bridges/tunnels.
use crate::{app::App, scene::World, spline_editor::Key};
use glam::DVec3;
use winit::keyboard::KeyCode;
#[derive(Clone, Copy)]
pub enum Command {
    Adjust(usize, f64),
    Edit(usize),
    Bridge,
    Tunnel,
    Apply,
    Plane,
    Hide,
    Commit,
    Close,
}
pub struct Window {
    pub key: Option<Key>,
    pub values: [f64; 5],
    pub input: Option<(usize, String, bool)>,
    pub rect: Option<[f32; 4]>,
    pub buttons: Vec<([f32; 4], Command)>,
    pub message: String,
}
impl Window {
    pub fn new(world: &World, key: Option<Key>, plane: Option<f64>) -> Self {
        let spline = key.and_then(|k| world.spline_edits.lock().current(k));
        let mut values = [0.0; 5];
        if let Some(s) = spline {
            let curve = omsi_geometry::SplineCurve::from_map(&s, glam::DVec2::ZERO);
            values = [
                s.pos[2],
                curve.end_point().z,
                s.grad_start,
                s.grad_end,
                s.pos[2],
            ];
        }
        if let Some(z) = plane {
            values[4] = z;
        }
        Self {
            key,
            values,
            input: None,
            rect: None,
            buttons: Vec::new(),
            message: "Set heights; use catalogue assets for deck, walls and portals".into(),
        }
    }
    pub fn commit(&mut self) -> bool {
        let Some((field, text, _)) = self.input.as_ref() else {
            return true;
        };
        let value = text.replace(',', ".").parse::<f64>();
        let Ok(value) = value else {
            self.message = "Enter a valid number".into();
            return false;
        };
        if !value.is_finite()
            || (if *field == 2 || *field == 3 {
                value.abs() > 20.0
            } else {
                !(-1000.0..=10000.0).contains(&value)
            })
        {
            self.message = "Height: -1000 to 10000 m; gradient: -20 to 20 percent".into();
            return false;
        }
        self.values[*field] = value;
        self.input = None;
        true
    }
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
    let curve = omsi_geometry::SplineCurve::from_map(&result, glam::DVec2::ZERO);
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
impl App {
    pub(crate) fn editor_open_construction(&mut self) {
        if self.net.lan.is_some() {
            self.service_msg = Some((
                "Construction tools are available in single-player".into(),
                5.0,
            ));
            return;
        }
        self.editor_terrain_finish();
        let (Some(ed), Some(world)) = (self.menus.editor.as_mut(), self.world.as_ref()) else {
            return;
        };
        ed.splines.finish_drag();
        ed.splines.cancel_connection();
        ed.splines.cancel_generation();
        ed.end_object_drag();
        ed.catalog = None;
        ed.sidewalk_window = None;
        ed.roadside_window = None;
        ed.junction_window = None;
        ed.traffic_window = None;
        ed.tile_window = None;
        ed.text_window = None;
        ed.placing_asset = None;
        ed.terrain.active = false;
        ed.construction_window = Some(Window::new(
            world,
            ed.splines.selected,
            ed.construction_plane,
        ));
        self.menus.editor_drag = false;
        self.input.keys.clear();
        self.input.buttons_held = (false, false);
        self.input.mouse_look = false;
    }
    pub(crate) fn editor_construction_command(&mut self, command: Command) {
        let (Some(ed), Some(world)) = (self.menus.editor.as_mut(), self.world.as_ref()) else {
            return;
        };
        let Some(w) = ed.construction_window.as_mut() else {
            return;
        };
        if matches!(command, Command::Close) {
            ed.construction_window = None;
            return;
        }
        if !w.commit() {
            return;
        }
        match command {
            Command::Edit(f) => w.input = Some((f, format!("{:.2}", w.values[f]), true)),
            Command::Adjust(f, d) => {
                let limit = if f == 2 || f == 3 {
                    (-20.0, 20.0)
                } else {
                    (-1000.0, 10000.0)
                };
                w.values[f] = (w.values[f] + d).clamp(limit.0, limit.1);
            }
            Command::Plane => {
                ed.construction_plane = if ed.construction_plane.is_some() {
                    None
                } else {
                    Some(w.values[4])
                }
            }
            Command::Hide => ed.hide_terrain = !ed.hide_terrain,
            Command::Bridge | Command::Tunnel => {
                let Some(key) = w.key else {
                    w.message = "Select a road first".into();
                    return;
                };
                let Some(s) = world.spline_edits.lock().current(key) else {
                    return;
                };
                let origin =
                    glam::DVec2::new(key.0 .0 as f64, key.0 .1 as f64) * omsi_map::tile_size();
                let c = omsi_geometry::SplineCurve::from_map(&s, origin);
                let p = c.point_at(0.0);
                let q = c.end_point();
                let (Some(a), Some(b)) = (
                    world.editor_terrain_height(p.x, p.y),
                    world.editor_terrain_height(q.x, q.y),
                ) else {
                    w.message = "Load terrain at both road ends first".into();
                    return;
                };
                let offset = if matches!(command, Command::Bridge) {
                    5.0
                } else {
                    -5.0
                };
                w.values = [a + offset, b + offset, 0.0, 0.0, a + offset];
                w.message="Preset: 5 m above/below terrain; adjust heights and check ramps before applying".into();
            }
            Command::Apply => {
                let Some(key) = w.key else {
                    w.message = "Select a road first; work plane also works in object mode".into();
                    return;
                };
                match ed.splines.apply_height_profile(world, key, w.values) {
                    Ok(()) => {
                        w.message="Height profile applied · Check linked ends · Ctrl+Z undo · Ctrl+S save".into();
                        self.editor_reload_splines();
                        return;
                    }
                    Err(e) => w.message = e,
                }
            }
            Command::Commit | Command::Close => {}
        }
        if ed.construction_plane.is_some() {
            ed.construction_plane = Some(w.values[4]);
        }
    }
    pub(crate) fn editor_construction_mouse(&mut self, pressed: bool) -> bool {
        let Some(w) = self
            .menus
            .editor
            .as_ref()
            .and_then(|e| e.construction_window.as_ref())
        else {
            return false;
        };
        if pressed {
            if let Some((_, c)) = w.buttons.iter().find(|(r, _)| {
                self.input.cursor.0 >= r[0]
                    && self.input.cursor.0 <= r[2]
                    && self.input.cursor.1 >= r[1]
                    && self.input.cursor.1 <= r[3]
            }) {
                let c = *c;
                self.editor_construction_command(c);
            }
        }
        true
    }
    pub(crate) fn editor_construction_input(
        &mut self,
        code: Option<KeyCode>,
        text: Option<&str>,
        pressed: bool,
    ) -> bool {
        if self.menus.game_menu.is_some() {
            return false;
        }
        if !self
            .menus
            .editor
            .as_ref()
            .is_some_and(|e| e.construction_window.is_some())
        {
            return false;
        }
        if matches!(
            code,
            Some(
                KeyCode::ControlLeft
                    | KeyCode::ControlRight
                    | KeyCode::ShiftLeft
                    | KeyCode::ShiftRight
            )
        ) {
            if let Some(k) = code {
                if pressed {
                    self.input.keys.insert(k);
                } else {
                    self.input.keys.remove(&k);
                }
            }
            return true;
        }
        let ctrl = self.input.keys.contains(&KeyCode::ControlLeft)
            || self.input.keys.contains(&KeyCode::ControlRight);
        if pressed
            && ctrl
            && !self
                .menus
                .editor
                .as_ref()
                .unwrap()
                .construction_window
                .as_ref()
                .unwrap()
                .input
                .is_some()
        {
            match code {
                Some(KeyCode::KeyZ) => {
                    self.editor_action(crate::editor::Action::SplineUndo);
                    return true;
                }
                Some(KeyCode::KeyS) => {
                    self.editor_action(crate::editor::Action::Save);
                    return true;
                }
                _ => {}
            }
        }
        let Some(w) = self
            .menus
            .editor
            .as_mut()
            .and_then(|e| e.construction_window.as_mut())
        else {
            return false;
        };
        if construction_camera_input(w, code, pressed, ctrl, &mut self.input.keys) {
            return true;
        }
        if !pressed {
            return true;
        }
        if code == Some(KeyCode::Escape) {
            if w.input.is_some() {
                w.input = None;
            } else {
                self.editor_construction_command(Command::Close);
            }
            return true;
        }
        if code == Some(KeyCode::Enter) {
            if w.input.is_some() {
                self.editor_construction_command(Command::Commit);
            } else {
                self.editor_construction_command(Command::Apply);
            }
            return true;
        }
        if let Some((_, value, replace)) = w.input.as_mut() {
            match code {
                Some(KeyCode::Backspace) => {
                    if *replace {
                        value.clear();
                    } else {
                        value.pop();
                    }
                    *replace = false;
                }
                Some(KeyCode::Delete) => {
                    value.clear();
                    *replace = false;
                }
                _ => {
                    if let Some(text) = text {
                        let filtered: String = text
                            .chars()
                            .filter(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | ',' | '.'))
                            .collect();
                        if !filtered.is_empty() {
                            if *replace {
                                value.clear();
                                *replace = false;
                            }
                            if value.len() + filtered.len() <= 24 {
                                value.push_str(&filtered);
                            }
                        }
                    }
                }
            }
        }
        true
    }
}
/// The modeless height panel only captures movement while entering a number.
/// Key releases always clear flight keys, including after a field gained focus.
fn construction_camera_input(window: &Window, code: Option<KeyCode>, pressed: bool, ctrl: bool,
    keys: &mut hashbrown::HashSet<KeyCode>) -> bool {
    let Some(code) = code else { return false; };
    if !matches!(code, KeyCode::KeyW | KeyCode::KeyA | KeyCode::KeyS | KeyCode::KeyD | KeyCode::KeyQ | KeyCode::KeyE | KeyCode::Space) {return false;}
    if !pressed {keys.remove(&code);return true;}
    if window.input.is_some() || ctrl {return false;}
    keys.insert(code);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn construction_panel_allows_flight_except_during_number_entry_and_clears_releases() {
        let mut editor=crate::editor::Editor::default();
        editor.construction_window=Some(Window {key:None,values:[0.0;5],input:None,rect:None,buttons:Vec::new(),message:String::new()});
        let mut keys=hashbrown::HashSet::new();
        let camera=[KeyCode::KeyW,KeyCode::KeyA,KeyCode::KeyS,KeyCode::KeyD,KeyCode::KeyQ,KeyCode::KeyE,KeyCode::Space];
        for code in camera {
            assert!(construction_camera_input(editor.construction_window.as_ref().unwrap(),Some(code),true,false,&mut keys));
            assert!(crate::editor::camera_key_pressed(&keys,Some(&editor),code));
        }
        editor.construction_window.as_mut().unwrap().input=Some((0,"12".into(),true));
        for code in camera {
            assert!(!crate::editor::camera_key_pressed(&keys,Some(&editor),code));
            assert!(!construction_camera_input(editor.construction_window.as_ref().unwrap(),Some(code),true,false,&mut keys));
            assert!(construction_camera_input(editor.construction_window.as_ref().unwrap(),Some(code),false,false,&mut keys));
        }
        assert!(keys.is_empty());
        editor.construction_window.as_mut().unwrap().input=None;
        assert!(!construction_camera_input(editor.construction_window.as_ref().unwrap(),Some(KeyCode::KeyS),true,true,&mut keys));
        assert!(!keys.contains(&KeyCode::KeyS));
        assert!(!construction_camera_input(editor.construction_window.as_ref().unwrap(),Some(KeyCode::KeyI),true,false,&mut keys));
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
    fn creating_a_tunnel_spline_keeps_the_work_plane_height() {
        let dir=std::env::temp_dir().join(format!("editor-plane-test-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("global.cfg"),"[name]\nPlane test\n").unwrap();
        let world=World::open(&dir,&dir.join("global.cfg"),20261012).unwrap();
        world.data.update(|d| { std::sync::Arc::make_mut(&mut d.ground).terrains.insert((0,0),std::sync::Arc::new(omsi_map::Terrain::flat())); });
        let at=DVec3::new(10.0,20.0,-5.0);
        let mut ed=crate::spline_editor::SplineEditor::default();ed.choose_file("example.sli".into());
        ed.generate_on_plane(&world,Some(at));
        assert_eq!(ed.markers(&world),vec![at]);
        ed.cancel_generation();ed.generate(&world,Some(at));
        assert!((ed.markers(&world)[0].z-0.05).abs()<1e-9);
        std::fs::remove_dir_all(dir).unwrap();
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
        let c = omsi_geometry::SplineCurve::from_map(&p, glam::DVec2::ZERO);
        assert!((c.end_point().z - 10.0).abs() < 1e-8);
        assert!(c.slope_at(0.0).abs() < 1e-8);
        assert!(c.slope_at(100.0).abs() < 1e-8);
        assert_eq!(p.pos[..2], s.pos[..2]);
        assert!(p.terrain_align.is_none() && !p.terrain_align_flag);
        assert!(height_profile(&s, [0.0, 20.0, 0.0, 0.0, 0.0]).is_err());
        assert!(height_profile(&s, [f64::NAN, 0.0, 0.0, 0.0, 0.0]).is_err());
    }
}

#[cfg(test)]
mod visual_tests {
    use super::*;
    #[test]
    #[ignore = "requires graphics adapter; creates editor panel screenshots"]
    fn construction_panel_fits_and_renders_in_german_and_english() {
        let instance = crate::graphics_instance();
        let mut renderer = pollster::block_on(omsi_render::Renderer::new_with(
            &instance,
            None,
            Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            omsi_render::RenderOptions {
                msaa: 1,
                ssao: false,
                fxaa: false,
                shadow_size: 1024,
                ..Default::default()
            },
        ))
        .unwrap();
        let mut editor = crate::editor::Editor::default();
        editor.construction_window = Some(Window {
            key: Some(((0, 0), 1)),
            values: [35.0, 40.0, 0.0, 0.0, 30.0],
            input: None,
            rect: None,
            buttons: Vec::new(),
            message: "Set heights; use catalogue assets for deck, walls and portals".into(),
        });
        let camera = omsi_render::Camera {
            position: DVec3::new(0.0, 0.0, 10.0),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            fov_deg: 60.0,
            near: 0.1,
            far: 1000.0,
        };
        for lang in ["DEU", "ENG"] {
            crate::ui_language(lang);
            for (width, height) in [(1280, 720), (800, 600)] {
                let mut scene = renderer.new_scene();
                let mut ui = crate::ui::Ui::new().unwrap();
                ui.draw_construction_window(
                    &renderer,
                    &mut scene,
                    Some(&mut editor),
                    1.0,
                    [0.0, 0.0, width as f32, height as f32],
                    (-1.0, -1.0),
                );
                let w = editor.construction_window.as_ref().unwrap();
                let panel = w.rect.unwrap();
                assert!(panel[2] <= width as f32 && panel[3] <= height as f32);
                assert_eq!(w.buttons.len(), 21);
                for (button, _) in &w.buttons {
                    assert!(
                        button[0] >= panel[0]
                            && button[1] >= panel[1]
                            && button[2] <= panel[2]
                            && button[3] <= panel[3]
                    );
                }
                let old_buttons=w.buttons.clone();
                assert!(ui.editor_window_drag_press((panel[0]+5.0,panel[1]+5.0)));
                assert!(ui.editor_dock_move((width as f32-5.0,height as f32-5.0)));
                assert!(ui.end_editor_dock_drag());
                scene.overlays.clear();
                ui.draw_construction_window(&renderer,&mut scene,Some(&mut editor),1.0,
                    [0.0,0.0,width as f32,height as f32],(-1.0,-1.0));
                let w=editor.construction_window.as_ref().unwrap();
                let moved=w.rect.unwrap();
                assert!(moved[0]>panel[0]);
                for (index,((old,_),(new,_))) in old_buttons.iter().zip(&w.buttons).enumerate() {
                    assert!((new[0]-old[0]-(moved[0]-panel[0])).abs()<0.01);
                    assert!((new[1]-old[1]-(moved[1]-panel[1])).abs()<0.01);
                    let cursor=((new[0]+new[2])*0.5,(new[1]+new[3])*0.5);
                    assert_eq!(w.buttons.iter().position(|(r,_)|cursor.0>=r[0]&&cursor.0<=r[2]&&cursor.1>=r[1]&&cursor.1<=r[3]),Some(index));
                }
                let rgba = renderer
                    .render_to_image(
                        &mut scene,
                        width,
                        height,
                        &camera,
                        &omsi_render::Lighting {
                            shadows: false,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                image::save_buffer(
                    format!("/tmp/editor-construction-{lang}-{width}.png"),
                    &rgba,
                    width,
                    height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
        let mut scene = renderer.new_scene();
        let mesh = renderer.add_mesh(
            &mut scene,
            &omsi_geometry::MeshData {
                positions: vec![
                    glam::Vec3::new(-100.0, -100.0, 0.0),
                    glam::Vec3::new(100.0, -100.0, 0.0),
                    glam::Vec3::new(100.0, 100.0, 0.0),
                    glam::Vec3::new(-100.0, 100.0, 0.0),
                ],
                normals: vec![glam::Vec3::Z; 4],
                uvs: vec![glam::Vec2::ZERO; 4],
                indices: vec![0, 1, 2, 0, 2, 3],
                ranges: vec![(0, 6, 0)],
                one_sided: false,
                ..Default::default()
            },
        );
        let blue = renderer.add_material(
            &mut scene,
            None,
            omsi_render::AlphaMode::Opaque,
            [0.0, 0.0, 1.0, 1.0],
            true,
        );
        let red = renderer.add_material(
            &mut scene,
            None,
            omsi_render::AlphaMode::Opaque,
            [1.0, 0.0, 0.0, 1.0],
            true,
        );
        let terrain = renderer.add_instance(
            &mut scene,
            mesh,
            DVec3::ZERO,
            glam::Mat4::IDENTITY,
            vec![blue],
        );
        scene.instances[terrain].render_phase = omsi_render::RenderPhase::Terrain;
        renderer.add_instance(
            &mut scene,
            mesh,
            -DVec3::Z * 3.0,
            glam::Mat4::IDENTITY,
            vec![red],
        );
        let camera = omsi_render::Camera {
            pitch: -90.0,
            ..camera
        };
        for hidden in [false, true, false] {
            scene.editor_hide_terrain = hidden;
            let rgba = renderer
                .render_to_image(
                    &mut scene,
                    64,
                    64,
                    &camera,
                    &omsi_render::Lighting {
                        shadows: false,
                        fog_density: 0.0,
                        ..Default::default()
                    },
                )
                .unwrap();
            let pixel = &rgba[(32 * 64 + 32) * 4..][..3];
            let want = if hidden { 0 } else { 2 };
            assert!(
                pixel[want] > 200 && pixel[2 - want] < 20,
                "terrain hidden={hidden}: {pixel:?}"
            );
            assert!(
                scene.instances[terrain].visible,
                "inspection must not change instance visibility"
            );
        }
        crate::ui_language("ENG");
    }
}
