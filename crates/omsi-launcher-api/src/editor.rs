//! Independent editor entry points and read-only update checks.
use crate::{arg, err, Answer, Platform};
use omsi_launcher_lib as core;
use omsi_platform::editor_updates::{Channel, ManagedUpdate};
use omsi_platform::updater::Status;
use serde_json::{json, Value};
use std::sync::Mutex;

fn idle() -> Result<(), String> {
    if core::instances::list().iter().any(|i| i.running) {
        return Err("Please end the current session before starting a map in the editor.".into());
    }
    Ok(())
}

pub fn open(p: &dyn Platform, map: String) -> Answer {
    idle()?;
    if !core::list_maps().map_err(err)?.iter().any(|m| m.file == map) {
        return Err("Map no longer available. Refresh the map list.".into());
    }
    let duty = core::Duty { map, editor: true, ..Default::default() };
    let launched = core::launch(&duty).map_err(err)?;
    p.launched();
    crate::to_json(launched)
}

pub fn create(args: &Value) -> Answer {
    idle()?;
    let content = core::content_dir().ok_or("Please configure Setup first.")?;
    let map = core::editor_maps::create(&content, &core::root_dir().map_err(err)?,
        &arg::<String>(args,"name")?, &arg::<String>(args,"author")?, &arg::<String>(args,"description")?).map_err(err)?;
    Ok(json!({ "map": map }))
}

static CHECKS: Mutex<Option<[ManagedUpdate;2]>> = Mutex::new(None);

pub fn updates(check: Option<usize>) -> Answer {
    if !omsi_platform::STANDALONE_EDITOR { return Err("Not an editor package".into()); }
    if check.is_some_and(|i|i>1) { return Err("Unknown update channel".into()); }
    let mut held = CHECKS.lock().unwrap_or_else(|e|e.into_inner());
    let checks = held.get_or_insert_with(||[ManagedUpdate::new(Channel::Game), ManagedUpdate::new(Channel::Editor)]);
    if let Some(i) = check { checks[i].check(); }
    Ok(json!(checks.iter().map(|u| {
        let installation = u.installation();
        let (state, message, release) = match u.status() {
            Status::Idle => ("idle", String::new(), None),
            Status::Checking => ("checking", String::new(), None),
            Status::UpToDate => ("up_to_date", String::new(), None),
            Status::Available(r) | Status::Downloaded(r) => ("available", r.version, Some(r.page)),
            Status::Failed(message) => ("failed", message, None),
            _ => ("busy", String::new(), None),
        };
        json!({"title":u.channel.title(),"state":state,"message":message,"current":installation.current,
            "path":installation.place.map(|p|p.dir.to_string_lossy().into_owned()), "release":release})
    }).collect::<Vec<_>>()))
}
