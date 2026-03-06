use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;

pub struct GlobalStats {
    pub start_time: DateTime<Utc>,
    pub total_checks: u64,
    pub total_failures: u64,
}

static STATS: Lazy<Arc<Mutex<GlobalStats>>> = Lazy::new(|| {
    Arc::new(Mutex::new(GlobalStats {
        start_time: Utc::now(),
        total_checks: 0,
        total_failures: 0,
    }))
});

pub fn record_check(success: bool) {
    let mut stats = STATS.lock().unwrap();
    stats.total_checks += 1;
    if !success {
        stats.total_failures += 1;
    }
}

pub fn get_report() -> String {
    let stats = STATS.lock().unwrap();
    let uptime = Utc::now() - stats.start_time;
    let days = uptime.num_days();
    let hours = uptime.num_hours() % 24;
    let minutes = uptime.num_minutes() % 60;

    format!(
        "📊 Daemon Stats:\n\
         - Uptime: {}d {}h {}m\n\
         - Total Checks: {}\n\
         - Failures Detected: {}\n\
         - Status: Active",
        days, hours, minutes, stats.total_checks, stats.total_failures
    )
}
