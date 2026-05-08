use tokio::time::Instant;

pub trait HealthReporter: Send + Sync {
    fn status(&self) -> &'static str;
    fn uptime_secs(&self) -> u64;
}

pub struct UptimeHealthReporter {
    start_time: Instant,
}

impl UptimeHealthReporter {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

impl HealthReporter for UptimeHealthReporter {
    fn status(&self) -> &'static str {
        "healthy"
    }

    fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
