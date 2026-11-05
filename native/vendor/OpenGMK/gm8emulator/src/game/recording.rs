use crate::game::{replay::Replay, Game};
use std::path::PathBuf;

impl Game {
    pub fn record(&mut self, _project_path: PathBuf, _pause_immediately: bool, _start_save_path: Option<&PathBuf>) {
        let _ = Replay::new(0, 0);
    }
}
