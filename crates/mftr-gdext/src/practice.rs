//! The champion creator's Test (CR3, 20 §10): a practice server in this process, a duel against
//! a bot with the champion just made. `MftrPractice.start(file)` loads the champion file and
//! returns the address to join; dropping it (or `stop`) ends the server.

use godot::prelude::*;
use mftr_server::customs::{ChampionPolicy, Customs};
use mftr_server::local::LocalServer;
use mftr_server::{Scenario, ServerConfig};

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrPractice {
    base: Base<RefCounted>,
    server: Option<LocalServer>,
    error: String,
}

#[godot_api]
impl MftrPractice {
    #[func]
    fn create() -> Gd<MftrPractice> {
        Gd::from_init_fn(|base| MftrPractice { base, server: None, error: String::new() })
    }

    /// Start a practice duel for the champion in `path` (a champion file): the address to join,
    /// or "" (see `error`).
    #[func]
    fn start(&mut self, path: GString) -> GString {
        self.server = None;
        let text = match std::fs::read_to_string(path.to_string()) {
            Ok(t) => t,
            Err(e) => {
                self.error = e.to_string();
                return GString::new();
            }
        };
        let customs = match Customs::load(vec![text]) {
            Ok(c) => c,
            Err(e) => {
                self.error = e;
                return GString::new();
            }
        };
        // The Duel Sandbox with one bot to fight (20 §10's first practice room).
        let cfg = ServerConfig {
            scenario: Scenario::Duel,
            bots: 1,
            max_players: 2,
            customs,
            champions: ChampionPolicy::Both,
            ..ServerConfig::default()
        };
        match LocalServer::start(cfg) {
            Ok(s) => {
                let address = GString::from(s.address());
                self.server = Some(s);
                address
            }
            Err(e) => {
                self.error = e.to_string();
                GString::new()
            }
        }
    }

    /// Why the last `start` failed.
    #[func]
    fn error(&self) -> GString {
        GString::from(self.error.as_str())
    }

    #[func]
    fn stop(&mut self) {
        self.server = None;
    }

    #[func]
    fn running(&self) -> bool {
        self.server.is_some()
    }
}
