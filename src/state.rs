use std::collections::HashMap;
use std::time::Instant;

const REMINDERS: &[u64] = &[1800, 3600, 7200, 10800, 18000];

pub struct SiteState {
    pub is_up: bool,
    pub down_since: Option<Instant>,
    pub reminders_sent: Vec<u64>,
    pub last_message: Option<String>,
}

pub struct Tracker {
    sites: HashMap<String, SiteState>,
}

impl Tracker {
    pub fn new() -> Self {
        Self {
            sites: HashMap::new(),
        }
    }

    pub fn get_last_message(&self, id: &str) -> Option<String> {
        self.sites.get(id).and_then(|s| s.last_message.clone())
    }

    pub fn update(&mut self, id: &str, display: &str, is_up: bool) -> Option<String> {
        let now = Instant::now();
        let state = self.sites.entry(id.into()).or_insert(SiteState {
            is_up: true,
            down_since: None,
            reminders_sent: vec![],
            last_message: None,
        });

        let mut msg = None;

        if state.is_up && !is_up {
            state.is_up = false;
            state.down_since = Some(now);
            state.reminders_sent.clear();
            msg = Some(format!("❌ {} is DOWN", display));
        } else if !state.is_up && !is_up {
            if let Some(start) = state.down_since {
                let elapsed = now.duration_since(start).as_secs();
                for &r in REMINDERS {
                    if elapsed >= r && !state.reminders_sent.contains(&r) {
                        state.reminders_sent.push(r);
                        msg = Some(format!("⏰ {} still DOWN ({} min)", display, elapsed / 60));
                        break;
                    }
                }
            }
        } else if !state.is_up && is_up {
            state.is_up = true;
            state.down_since = None;
            state.reminders_sent.clear();
            msg = Some(format!("✅ {} recovered", display));
        }

        if msg.is_some() {
            state.last_message = msg.clone();
        }
        
        msg
    }
}
