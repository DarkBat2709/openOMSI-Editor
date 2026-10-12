//! Continue the live CPU state after rebuilding a scene, without running init again.
use super::*;
pub struct VehicleReloadState(VehicleInstance);
impl VehicleInstance {
    pub fn into_reload_state(self) -> VehicleReloadState {VehicleReloadState(self)}
    pub fn restore_reload_state(&mut self, saved:VehicleReloadState) -> bool {
        let old=saved.0;
        if self.ty.def.path!=old.ty.def.path || self.ty.program.var_names!=old.ty.program.var_names
            || self.ty.program.str_var_names!=old.ty.program.str_var_names {return false;}
        // Engine-added variables are absent from an ordinary situation record. Rebind
        // them by name, because their slot order can differ in the fresh instance.
        for (name,id) in &old.var_index {
            self.set_engine_var(name,old.state.vars[*id as usize]);
        }
        self.state.str_vars=old.state.str_vars;
        self.vm=old.vm;
        self.pending_destination=old.pending_destination;
        self.position=old.position;self.heading=old.heading;self.pitch=old.pitch;self.bank=old.bank;
        self.physics=old.physics;self.rigid=old.rigid;
        if let Some(rb)=self.rigid.as_mut() {
            // New map obstacles must be checked against the continued bus pose.
            rb.spawned_inside=None;rb.wheel_impacts.clear();
        }
        self.seat=old.seat;self.a_trans=old.a_trans;self.animators=old.animators;
        self.light_fade=old.light_fade;self.driven_km=old.driven_km;
        self.km_started=old.km_started;self.cabin_air=old.cabin_air;self.dirt=old.dirt;
        self.crashes=old.crashes;self.last_impact=old.last_impact;
        self.host.clock=old.host.clock;self.host.clock.timegap=0.0;
        self.host.wear_lifespan=old.host.wear_lifespan;
        self.host.km_base=old.host.km_base;self.host.auto_clutch=old.host.auto_clutch;
        self.host.humans_count=old.host.humans_count;self.host.schedule_active=old.host.schedule_active;
        self.host.humans_on_path_link=old.host.humans_on_path_link;
        self.host.humans_on_seat=old.host.humans_on_seat;
        self.host.change_coins=old.host.change_coins;
        self.host.tt_line=old.host.tt_line;self.host.tt_delay=old.host.tt_delay;
        self.host.tt_stops=old.host.tt_stops;self.host.tt_stop_ids=old.host.tt_stop_ids;
        self.host.tt_terminus_index=old.host.tt_terminus_index;self.host.tt_busstop_index=old.host.tt_busstop_index;
        // Keep the newly attached map samplers, font library, HTML and scene handles.
        // Init's queued collision/sound events must not fire after restoring old state.
        self.collided=false;self.last_crash=0.0;self.dynamic_impacts.clear();
        self.host.coll_energy=0.0;self.host.fired_triggers.clear();
        self.host.fired_trigger_vars.clear();self.host.fired_file_triggers.clear();
        for t in &mut self.text_textures {t.last_text=None;}
        for part in &mut self.trailers {for t in &mut part.text_textures {t.last_text=None;}}
        self.update_visuals(0.0);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_reload_keeps_wear_engine_gearbox_and_wheel_state_without_repairing() {
        for damage in [0.0,1.0] {
            let mut old=super::super::tests::restored_display_vehicle();
            old.set_engine_var("engine_failure_general",damage);
            old.set_engine_var("antrieb_failure_general",damage);
            old.set_engine_var("engine_on",1.0);
            old.set_engine_var("wearlifespan",1000.0);
            old.host.wear_lifespan=1000.0;
            old.physics.speed=5.0;old.physics.steer_deg=12.0;
            old.set_odometer_km(12345.5);
            let mut fresh=VehicleInstance::new(old.ty.clone(),VehicleHost::new(Default::default()));
            fresh.set_engine_var("unrelated",44.0);
            fresh.set_engine_var("engine_failure_general",1.0-damage);
            fresh.set_engine_var("engine_on",0.0);
            assert!(fresh.restore_reload_state(old.into_reload_state()));
            assert_eq!(fresh.var("engine_failure_general"),Some(damage));
            assert_eq!(fresh.var("antrieb_failure_general"),Some(damage));
            assert_eq!(fresh.var("engine_on"),Some(1.0));
            assert_eq!(fresh.var("wearlifespan"),Some(1000.0));
            assert_eq!(fresh.host.wear_lifespan,1000.0);
            assert_eq!(fresh.physics.speed,5.0);assert_eq!(fresh.physics.steer_deg,12.0);
            assert_eq!(fresh.odometer_km(),12345.5);
            assert_eq!(fresh.last_crash,0.0);assert!(!fresh.collided);
        }
    }
}
