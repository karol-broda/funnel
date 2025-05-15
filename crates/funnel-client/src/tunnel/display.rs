use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use console::Style;
use indicatif::{ProgressBar, ProgressStyle};

const SPINNER_TICK_MS: u64 = 80;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct TunnelDisplay {
    pb: ProgressBar,
    requests: AtomicU64,
    started: Instant,
}

pub struct RequestResult {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration: Duration,
}

impl TunnelDisplay {
    pub fn new() -> Self {
        let pb = ProgressBar::new_spinner();
        let style = ProgressStyle::default_spinner()
            .tick_strings(SPINNER_FRAMES)
            .template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());
        pb.set_style(style);
        pb.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        pb.set_message("waiting for requests...");

        Self {
            pb,
            requests: AtomicU64::new(0),
            started: Instant::now(),
        }
    }

    pub fn set_message(&self, msg: &str) {
        self.pb.set_message(msg.to_string());
    }

    pub fn println(&self, msg: &str) {
        self.pb.println(msg);
    }

    pub fn log_request(&self, result: &RequestResult) {
        self.requests.fetch_add(1, Ordering::Relaxed);

        let duration = format_duration(result.duration);

        // status 0 means a raw stream connection (TCP/TLS), not HTTP
        if result.status == 0 {
            let style = Style::new().cyan();
            self.pb.println(format!(
                "{} {} {} {}",
                result.method,
                result.path,
                style.apply_to("connected"),
                duration,
            ));
            return;
        }

        let status_style = match result.status {
            200..=299 => Style::new().green(),
            300..=499 => Style::new().yellow(),
            _ => Style::new().red(),
        };

        let status = status_style.apply_to(result.status);

        self.pb.println(format!(
            "{} {} {} {}",
            result.method, result.path, status, duration
        ));
    }

    pub fn finish(&self) {
        let reqs = self.requests.load(Ordering::Relaxed);
        let uptime = self.started.elapsed();

        self.pb.finish_and_clear();

        println!();
        println!("session summary");
        println!("  requests   {reqs}");
        println!("  uptime     {}", format_uptime(uptime));
    }
}

fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

fn format_uptime(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{hours}h {mins}m {secs}s")
    } else if mins > 0 {
        format!("{mins}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_millis() {
        assert_eq!(format_duration(Duration::from_millis(12)), "12ms");
        assert_eq!(format_duration(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_millis(1200)), "1.2s");
        assert_eq!(format_duration(Duration::from_secs(5)), "5.0s");
    }

    #[test]
    fn format_uptime_units() {
        assert_eq!(format_uptime(Duration::from_secs(45)), "45s");
        assert_eq!(format_uptime(Duration::from_secs(125)), "2m 5s");
        assert_eq!(format_uptime(Duration::from_secs(3661)), "1h 1m 1s");
    }
}
