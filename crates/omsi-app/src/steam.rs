//! Steamworks API layer for displaying in-game status using steam's presence

pub struct Steam {
    pub client: steamworks::Client
}

impl Steam {
    pub fn start() -> Option<Steam> {
        // Unit tests construct multiple Apps concurrently; Steam is process-global.
        if cfg!(test) { return None; }
        if let Ok(client) = steamworks::Client::init_app(252530) {
            log::info!("Steam: Connected");
            
            Some(Steam {
                client
            })
        } else {
            None
        }
    }
}
