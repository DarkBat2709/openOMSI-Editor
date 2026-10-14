//! Reload a saved map, keeping the camera and the current driving session.

/// CPU-only continuation data. Scene-local renders are always recreated.
pub(crate) struct DrivingReload {
    pub(crate) view: String,
    pub(crate) ego: bool,
    pub(crate) camera_choice: Option<((usize,usize),Option<usize>,glam::Vec3)>,
    pub(crate) vehicle: Option<omsi_sim::vehicle::VehicleReloadState>,
    pub(crate) original_args: crate::Args,
    pub(crate) duty: Option<crate::schedule::PlayerDuty>,
    pub(crate) duty_typed: bool,
    pub(crate) textures: Vec<(u32,u32,Vec<u8>)>,
    pub(crate) pose: Option<(glam::DVec3,f64,f32,f32)>,
    pub(crate) passengers: Option<omsi_sim::people::reload::PlayerPassengers>,
    pub(crate) rigid_pose: Option<(glam::DVec3,glam::Quat)>,
}

use crate::*;

/// Never discard the current world after a failed save or failed map preflight.
fn prepare_reload<T>(save:bool,write:impl FnOnce()->Result<(),String>,open:impl FnOnce()->Result<T,String>)->Result<T,String>{
    if save {write()?;}
    open()
}

impl App {
    #[cfg(target_os="android")]
    pub(crate) fn editor_reload_map(&mut self){
        self.service_msg=Some(("Reload map is available in the desktop editor".into(),5.0));
    }

    #[cfg(not(target_os="android"))]
    pub(crate) fn editor_reload_map(&mut self){
        if self.net.lan.is_some() || self.cam.starting.is_some(){
            self.service_msg=Some(("Reload map is available in the loaded single-player editor".into(),5.0));return;
        }
        if self.gfx.streamer.as_ref().is_some_and(|s| s.editor_rebuilding()) {
            self.service_msg=Some(("The edited tiles are still loading. Please reload the map once they are ready.".into(),5.0));return;
        }
        if self.renderer.is_none(){return;}
        let (Some(cam),Some(ed),Some(old))=(self.camera,self.menus.editor.as_ref(),self.world.clone())else{return;};
        let mode=(ed.spline_mode,ed.terrain.active);
        self.editor_terrain_finish();
        if let Some(ed)=self.menus.editor.as_mut(){ed.splines.finish_drag();ed.end_object_drag();}
        self.input.keys.clear();self.input.buttons_held=(false,false);self.input.mmb_held=false;
        self.menus.editor_drag=false;self.input.dragging=false;self.input.mouse_look=false;
        self.sync_look_hold();
        let save_label=omsi_ui::tr("Save & reload").into_owned();
        let discard_label=omsi_ui::tr("Discard changes & reload").into_owned();
        let result=rfd::MessageDialog::new().set_title(omsi_ui::tr("Reload map").as_ref())
            .set_description(omsi_ui::tr("The map will be reloaded from its saved state. Camera position and direction, the bus location, its passengers and the active duty are kept.\n\nSave or discard unsaved map changes first? Undo history and selections will be reset. Vehicles and AI will be rebuilt.").as_ref())
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(save_label.clone(),discard_label.clone(),omsi_ui::tr("Cancel").into_owned())).show();
        let save=match result {
            rfd::MessageDialogResult::Yes=>true,
            rfd::MessageDialogResult::No=>false,
            rfd::MessageDialogResult::Custom(s) if s==save_label=>true,
            rfd::MessageDialogResult::Custom(s) if s==discard_label=>false,
            _=>return,
        };
        let fresh=prepare_reload(save,||{
            let content=crate::startup::content_dir().ok_or("No writable content folder")?;
            self.menus.editor.as_ref().unwrap().save(&old,&self.args.map,&content,&self.args.root).map(|_|())
                .map_err(|e|format!("Save failed: {e}"))
        },||{
            omsi_cfg::content_changed();
            let path=omsi_cfg::resolve_path(&self.args.root,&self.args.map);
            let world=World::open(&self.args.root,&path,self.clock.date_code()).map_err(|e|format!("Could not open map: {e:#}"))?;
            *world.start_clock.lock()=self.clock.clone();
            Ok(Arc::new(world))
        });
        let world=match fresh{Ok(w)=>w,Err(e)=>{self.service_msg=Some((format!("Not reloaded: {e}"),15.0));return;}};
        // Prepare the ordinary situation-resume path entirely in memory. No save-game
        // file is written and failed preflight above leaves the current session intact.
        let mut resume_args=self.args.clone();
        if self.player.is_some() {
            let sit=crate::situation::build_situation(&self.args,&old,&self.clock,None,
                self.player.as_ref(),&self.session.placed,&cam,self.session.duty.as_ref(),"Editor reload");
            crate::situation::apply_situation_parsed(&sit,&mut resume_args);
        }
        resume_args.schedule=self.args.schedule;
        prepare_driving_args(&mut resume_args,self.player.is_some(),self.session.duty.is_some(),&self.clock);
        if let Some(p)=self.player.as_ref() {log_reload_bus("before",&p.vehicle);}
        let mut continuation=DrivingReload {
            view:self.view.clone(), ego:self.cam.ego,
            camera_choice:self.player.as_ref().map(|p|(p.cam_choice,p.cam_before_special,p.seat)),
            vehicle: None,
            passengers: self.session.humans.take().filter(|_| self.player.is_some()).map(|h| h.sim.into_player_passengers()),
            original_args: std::mem::replace(&mut self.args,resume_args),
            duty: self.session.duty.take(),
            duty_typed: self.player.as_ref().is_some_and(|p|p.duty_typed),
            textures: self.player.as_ref().map(|p|p.vehicle.host.script_textures.iter()
                .map(|t|(t.width,t.height,t.rgba.clone())).collect()).unwrap_or_default(),
            rigid_pose: self.player.as_ref().and_then(|p|p.vehicle.rigid.as_ref()).map(|rb|(rb.position,rb.orientation)),
            pose: self.player.as_ref().map(|p|(p.vehicle.position,p.vehicle.heading,p.vehicle.pitch,p.vehicle.bank)),
        };
        continuation.vehicle=self.player.take().map(|p|p.vehicle.into_reload_state());
        self.cam.editor_reload_driving=Some(continuation);
        let renderer=self.renderer.take().unwrap();
        let mut scene=renderer.new_scene();
        setup_sky(&self.args,&renderer,&mut scene,self.session.envir.as_ref(),self.session.weather.as_ref(), &mut self.session.sky.clouds);
        world.set_fast_texture_loads(true);
        world.set_texture_budget(texture_budget(&self.settings));
        let distance=self.args.view_distance.or_else(settings::view_distance).unwrap_or(1200.0).max(omsi_map::tile_size());
        let streamer=tiles::Streamer::new(world.clone(),&crate::app::start_centers(&self.args,&cam,Some(&world)),distance,700.0);
        // Every owner of scene-local GPU IDs must go with the old scene.
        self.gfx.streamer=None;self.session.traffic=None;self.session.schedule=None;self.session.humans=None;
        // Since 0.2.38 the displayed people and traffic live outside their simulations.
        // Their instance/texture IDs belong to the discarded scene too.
        self.gfx.sim_view=Default::default();
        self.session.timetable_book=None;
        self.sound.voices_waiting.clear();
        self.player=None;self.session.placed.clear();self.session.duty=None;self.session.duty_places=false;
        self.sound.soundscape=None;self.sound.ambience=None;self.sound.audio=None;self.menus.navigator=None;self.session.on_foot=None;
        self.menus.placing=None;self.menus.remote_added.clear();self.gfx.route_arrows=Default::default();
        self.session.rain=rain::Rain::new();self.session.spray=puddles::Spray::new();
        self.gfx.frozen_mirrors=None;self.gfx.mirror_hud=Default::default();self.input.touch.drop_gpu();
        self.ui=ui::Ui::new();self.menus.hud=Some(hud::Hud::new(&mut world.fonts.lock()));
        if let Some(mut plugins)=self.integrations.plugins.take(){plugins.finalize();}
        self.integrations.plugin_keys.clear();self.integrations.plugin_events.clear();self.integrations.plugin_panels=Default::default();
        self.integrations.plugin_events_ex.clear();self.integrations.plugin_voices.clear();
        self.integrations.plugin_seen=Default::default();
        self.menus.editor=None;self.menus.editor_paused=None;
        self.input.pressed_scenery_object=None;self.input.cruise=None;
        self.session.safe_pose=None;self.menus.hover_key=None;self.input.html_pressed=None;self.input.html_object_pressed=None;
        self.session.lamps_on=None;self.session.world_day=None;self.session.first_populate=true;
        self.session.populate_t=0.0;self.session.humans_populate_t=0.0;
        self.world=Some(world);self.gfx.streamer=Some(streamer);self.scene=Some(scene);self.renderer=Some(renderer);
        self.camera=Some(cam);self.cam.starting=Some(cam);self.cam.editor_reload_view=Some((cam,mode.0,mode.1));
        self.last=Instant::now();self.service_msg=Some(("Reloading map …".into(),10.0));
    }

    pub(crate) fn finish_editor_reload(&mut self, r: &omsi_render::Renderer, scene: &mut omsi_render::Scene){
        let Some((cam,spline,terrain))=self.cam.editor_reload_view.take()else{return;};
        let mut kept_bus=false;
        let mut kept_duty=false;
        let mut kept_passengers=0;
        let mut kept_waiting=0;
        let mut restored_view="free".to_string();
        let mut restored_ego=false;
        if let Some(saved)=self.cam.editor_reload_driving.take() {
            if let Some(p)=self.player.as_mut() {
                restored_view=saved.view;restored_ego=saved.ego;
                if let Some((choice,before,seat))=saved.camera_choice {
                    p.cam_choice=choice;p.cam_before_special=before;p.seat=seat;
                }
                if let Some(vehicle)=saved.vehicle {
                    if !p.vehicle.restore_reload_state(vehicle) {log::warn!("editor reload: live bus state incompatible with replacement model");}
                    if let Some(sounds)=p.sounds.as_mut() { sounds.resume_conditions(&|n| p.vehicle.var(n)); }
                    log_reload_bus("restored",&p.vehicle);
                }
                if let Some((at,heading,pitch,bank))=saved.pose {
                    p.vehicle.position=at;p.vehicle.heading=heading;p.vehicle.pitch=pitch;p.vehicle.bank=bank;
                    kept_bus=true;
                }
                if let (Some(rb),Some((position,orientation)))=(p.vehicle.rigid.as_mut(),saved.rigid_pose) {
                    rb.position=position;rb.orientation=orientation;
                }
                for (t,(width,height,rgba)) in p.vehicle.host.script_textures.iter_mut().zip(saved.textures) {
                    t.restore(width,height,rgba);
                }
                if let (Some(h), Some(passengers)) = (self.session.humans.as_mut(), saved.passengers) {
                    if let Some(w) = self.world.as_ref() {
                        h.stop_targets=self.session.schedule.as_ref().map(|s|s.stop_targets());
                        h.stop_names=self.session.schedule.as_ref().map(|s|s.stop_names());
                        h.center=p.vehicle.position;
                        (kept_passengers,kept_waiting)=h.restore_player_passengers_in_world(passengers,w.data().as_ref());
                        if let Some(m) = h.money.as_mut() { crate::money::restore_scene(m, w, r, scene); }
                        h.show_bodies(&mut self.gfx.sim_view.people, w, r, scene);
                    }
                    log::info!("editor reload: restored {kept_passengers} player passengers and {kept_waiting} waiting passengers");
                }
                p.duty_typed=saved.duty_typed;
                kept_duty=saved.duty.is_some();
                self.session.duty=saved.duty;
                self.session.duty_places=false;
                if let (Some(d),Some(w))=(self.session.duty.as_ref(),self.world.as_ref()) {
                    let _=crate::schedule_paper::update_vehicle(&mut p.vehicle,d,&mut w.fonts.lock());
                }
            }
            self.args=saved.original_args;
        }
        let mut editor=crate::editor::Editor::default();editor.spline_mode=spline;editor.terrain.active=terrain;
        self.menus.editor=Some(editor);self.camera=Some(cam);self.view=restored_view;self.cam.ego=restored_ego;
        self.input.keys.clear();self.input.buttons_held=(false,false);self.input.mmb_held=false;self.input.mouse_look=false;
        log::info!("editor reload complete: bus={kept_bus}, duty={kept_duty}, passengers={kept_passengers}, waiting={kept_waiting}");
        self.service_msg=Some((if !kept_bus && self.args.bus.is_some() {"Karte neu geladen · Bus konnte nicht wiederhergestellt werden; siehe game.log"}
            else if kept_duty {"Karte neu geladen · Busposition, Fahrgäste und Linienauftrag beibehalten"}
            else if kept_bus {"Karte neu geladen · Busposition beibehalten · Freie Fahrt"}
            else {"Karte neu geladen · Kameraposition beibehalten"}.into(),10.0));
    }
}

fn log_reload_bus(stage:&str, bus:&omsi_sim::VehicleInstance) {
    let vars:Vec<_>=["engine_on","engine_n","engine_failure_general","antrieb_failure_general",
        "elec_failure_general","wearlifespan","Gear_Error_dist","EDC_Error_dist"]
        .into_iter().filter_map(|n|bus.var(n).map(|v|(n,v))).collect();
    log::info!("editor reload bus {stage}: speed={:.3} m/s, maintenance={}, vars={vars:?}",
        bus.physics.speed,bus.host.wear_lifespan);
}

/// Keep a free drive free, and use the current clock rather than the launch clock.
fn prepare_driving_args(args:&mut Args,has_bus:bool,has_duty:bool,clock:&omsi_sim::SimClock){
    args.time=format!("0:0:{}",clock.time);
    args.date=Some(format!("{}-01-01",clock.year));args.day_of_year=Some(clock.day_of_year);
    args.autostart=false;args.situation=None;args.riders=0;
    // The live duty itself is restored after startup, avoiding a new assignment or
    // catch-up against the launch-time stop. The new schedule still rebuilds normally.
    args.line=None;args.tour=None;args.trip=None;args.duty_trip=None;
    args.situation_next_stop=None;
    if !has_bus {args.bus=None;args.spawn=None;args.situation_others.clear();}
    if has_duty {args.schedule=true;}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reload_keeps_existing_waiters_and_reconciles_moved_or_deleted_stops() {
        use omsi_sim::people::{PeopleSim,State};
        use omsi_sim::human::HumanType;
        let dir=std::env::temp_dir().join(format!("editor-waiting-reload-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("global.cfg"),"[name]\nWaiting test\n").unwrap();
        std::fs::write(dir.join("person.hum"),"[model]\nmodel.cfg\n").unwrap();
        std::fs::write(dir.join("model.cfg"),"").unwrap();
        let ty=Arc::new(HumanType::load(&dir.join("person.hum")).unwrap());
        for change in [0,1,2,3] {
            let world=World::open(&dir,&dir.join("global.cfg"),20261012).unwrap();
            world.data.update(|d| { std::sync::Arc::make_mut(&mut d.ground).terrains.insert((0,0),Arc::new(omsi_map::Terrain::flat())); });
            let at=glam::DVec3::new(10.0,10.0,0.0);
            world.data.update(|d| Arc::make_mut(&mut d.lists).bus_stops.push((123,at,0.0,"Test stop".into())));
            world.data.update(|d| Arc::make_mut(&mut d.objects).place((0,0),vec![(456,(glam::DVec3::new(11.0,10.0,0.0),[0.0;3]))]));
            world.data.update(|d| Arc::make_mut(&mut d.lists).waiting_places.push((456,glam::DVec3::new(11.0,11.0,0.0),0.0,0.0)));
            let mut old=PeopleSim::new(&dir,200);old.types=vec![ty.clone()];
            old.populate(world.data().as_ref(),at);
            let index=old.spawn_waiting(world.data().as_ref(),123).expect("synthetic stop must have a waiting place");
            let id=old.people[index].id;
            if let State::Pax(p)=&mut old.people[index].state {p.dest=Some("Hauptbahnhof".into());}
            let saved=old.into_player_passengers();
            if change==1 {world.data.update(|d| Arc::make_mut(&mut d.lists).bus_stops[0].1.x+=5.0);}
            if change==2 {world.data.update(|d| Arc::make_mut(&mut d.lists).bus_stops.clear());}
            if change==3 {world.data.update(|d| Arc::make_mut(&mut d.lists).bus_stops[0].1.x+=100.0);}
            let mut fresh=PeopleSim::new(&dir,200);fresh.center=at;
            let (riders,waiting)=fresh.restore_player_passengers_in_world(saved,world.data().as_ref());
            assert_eq!(riders,0);
            assert_eq!(waiting,if change<=1{1}else{0});
            if change<=1 {
                assert_eq!(fresh.people[0].id,id);
                let State::Pax(p)=&fresh.people[0].state else{panic!("waiter missing");};
                assert_eq!(p.dest.as_deref(),Some("Hauptbahnhof"));
                assert_eq!(p.task.name(),"WaitingForBus");assert!(p.bus.is_none());
                assert_eq!(fresh.stops[&123].taken,vec![true]);
                if change==1 {assert!((fresh.people[0].position.x-16.0).abs()<0.01);}
                fresh.populate(world.data().as_ref(),at);assert_eq!(fresh.people.len(),1);
                assert!(fresh.next_id>id);
            } else {assert!(fresh.people.is_empty());}
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test] fn reload_uses_current_clock_and_never_reassigns_a_duty(){
        let mut args=Args::parse_from(["openomsi","--line","137","--tour","4"]);
        let mut clock=omsi_sim::SimClock::default();clock.time=47823.75;clock.year=2026;clock.day_of_year=284;
        prepare_driving_args(&mut args,true,true,&clock);
        assert_eq!(crate::cli::parse_time(&args.time),clock.time);
        assert_eq!(args.day_of_year,Some(284));assert!(args.schedule);
        assert!(args.line.is_none()&&args.tour.is_none()&&args.trip.is_none());assert!(!args.autostart);
    }
    #[test] fn reload_without_player_does_not_spawn_launch_bus(){
        let mut args=Args::parse_from(["openomsi","--bus","example.bus"]);
        prepare_driving_args(&mut args,false,false,&omsi_sim::SimClock::default());
        assert!(args.bus.is_none()&&args.spawn.is_none());
    }

    #[test]fn failed_save_never_opens_or_discards_world(){
        let result:Result<(),String>=prepare_reload(true,||Err("write failed".into()),||panic!("must not reload"));
        assert_eq!(result.unwrap_err(),"write failed");
    }
    #[test]fn discard_does_not_write_and_save_precedes_open(){
        assert_eq!(prepare_reload(false,||panic!("must not save"),||Ok(7)).unwrap(),7);
        let order=std::cell::RefCell::new(Vec::new());
        prepare_reload(true,||{order.borrow_mut().push("save");Ok(())},||{order.borrow_mut().push("open");Ok(())}).unwrap();
        assert_eq!(*order.borrow(),["save","open"]);
    }
    #[test]fn open_failure_is_returned_before_commit(){
        let result:Result<(),String>=prepare_reload(false,||Ok(()),||Err("bad map".into()));
        assert_eq!(result.unwrap_err(),"bad map");
    }
}
