//! Reload a saved map into a new world, keeping the editor's camera.
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
            .set_description(omsi_ui::tr("The map will be reloaded from its saved state. Camera position and direction are kept.\n\nSave or discard unsaved map changes first? Undo history and selections will be reset. Vehicles and AI will be rebuilt.").as_ref())
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
        let renderer=self.renderer.take().unwrap();
        let mut scene=renderer.new_scene();
        setup_sky(&self.args,&renderer,&mut scene,self.session.envir.as_ref(),self.session.weather.as_ref());
        world.set_fast_texture_loads(true);
        world.set_texture_budget(texture_budget(&self.settings));
        let distance=self.args.view_distance.or_else(settings::view_distance).unwrap_or(1200.0).max(omsi_map::tile_size());
        let streamer=tiles::Streamer::new(world.clone(),&[cam.position],distance,700.0);
        // Every owner of scene-local GPU IDs must go with the old scene.
        self.gfx.streamer=None;self.session.traffic=None;self.session.schedule=None;self.session.humans=None;
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

    pub(crate) fn finish_editor_reload(&mut self){
        let Some((cam,spline,terrain))=self.cam.editor_reload_view.take()else{return;};
        let mut editor=crate::editor::Editor::default();editor.spline_mode=spline;editor.terrain.active=terrain;
        self.menus.editor=Some(editor);self.camera=Some(cam);self.view="free".into();self.cam.ego=false;
        self.input.keys.clear();self.input.buttons_held=(false,false);self.input.mmb_held=false;self.input.mouse_look=false;
        self.service_msg=Some(("Map reloaded · Camera position kept".into(),8.0));
    }
}

#[cfg(test)]
mod tests {
    use super::prepare_reload;
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
