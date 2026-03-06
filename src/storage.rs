use crate::app_state::AppState;
use std::fs;

const FILE: &str = "app_state.json";

pub fn load() -> AppState {
    if let Ok(content) = fs::read_to_string(FILE) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppState::default()
    }
}

pub fn save(state: &AppState) {
    let _ = fs::write(FILE, serde_json::to_string_pretty(state).unwrap());
}
