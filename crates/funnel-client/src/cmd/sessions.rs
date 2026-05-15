use funnel_core::api as endpoints;

use super::api;

fn format_timestamp(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub async fn list(
    server: &str,
    token: &str,
    all: bool,
    limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let mut path = format!("/sessions?limit={limit}");
    if all {
        path.push_str("&all=true");
    }

    let Some(sessions) =
        api::call_at(server, token, &endpoints::SESSIONS_LIST, &path, json).await?
    else {
        return Ok(());
    };

    if sessions.is_empty() {
        println!("no sessions");
        return Ok(());
    }

    for (i, s) in sessions.iter().enumerate() {
        let status = if s.disconnected_at.is_some() {
            "closed"
        } else {
            "active"
        };
        println!("{} ({})", s.tunnel_id, status);
        println!("  id:       {}", s.id);
        println!("  started:  {}", format_timestamp(&s.connected_at));
        if let Some(ref end) = s.disconnected_at {
            println!("  ended:    {}", format_timestamp(end));
        }
        println!("  requests: {}", s.requests);
        println!(
            "  traffic:  {} in / {} out",
            format_bytes(s.bytes_in),
            format_bytes(s.bytes_out)
        );
        if i < sessions.len() - 1 {
            println!();
        }
    }

    if sessions.len() > 1 {
        println!("\n{} sessions", sessions.len());
    }

    Ok(())
}

const KIB: i64 = 1024;
const MIB: i64 = 1024 * KIB;
const GIB: i64 = 1024 * MIB;

fn format_bytes(bytes: i64) -> String {
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
