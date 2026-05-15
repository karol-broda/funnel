use funnel_core::api as endpoints;

use super::api;

pub async fn run(server: &str, token: &str, json: bool) -> anyhow::Result<()> {
    let Some(tunnels) = api::call(server, token, &endpoints::TUNNELS_LIST, json).await? else {
        return Ok(());
    };

    if tunnels.is_empty() {
        println!("no active tunnels");
        return Ok(());
    }

    for (i, t) in tunnels.iter().enumerate() {
        println!("{}", t.id);
        println!("  uptime:   {}", format_duration(t.uptime_secs));
        println!("  requests: {}", t.stats.requests);
        println!(
            "  traffic:  {} in / {} out",
            format_bytes(t.stats.bytes_in),
            format_bytes(t.stats.bytes_out)
        );
        println!("  owner:    {}", t.owner_id);
        if let Some(ref team) = t.team_id {
            println!("  team:     {team}");
        }
        if i < tunnels.len() - 1 {
            println!();
        }
    }

    if tunnels.len() > 1 {
        println!("\n{} tunnels active", tunnels.len());
    }

    Ok(())
}

fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;

    if hours > 0 {
        format!("{hours}h{minutes:02}m{seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m{seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;

fn format_bytes(bytes: u64) -> String {
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_seconds() {
        assert_eq!(format_duration(45.0), "45s");
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(125.0), "2m05s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3661.0), "1h01m01s");
    }

    #[test]
    fn format_bytes_small() {
        assert_eq!(format_bytes(500), "500 B");
    }

    #[test]
    fn format_bytes_kib() {
        assert_eq!(format_bytes(2048), "2.0 KiB");
    }

    #[test]
    fn format_bytes_mib() {
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MiB");
    }

    #[test]
    fn format_bytes_gib() {
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }
}
