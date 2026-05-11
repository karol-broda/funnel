use funnel_core::protocol::PROTOCOL_VERSION;
use serde::Deserialize;

#[derive(Deserialize)]
struct TunnelSession {
    id: String,
    tunnel_id: String,
    connected_at: String,
    disconnected_at: Option<String>,
    bytes_in: i64,
    bytes_out: i64,
    requests: i64,
}

pub async fn list(server: &str, token: &str, all: bool, limit: u32) -> anyhow::Result<()> {
    let mut url = format!("{server}/api/v{PROTOCOL_VERSION}/sessions?limit={limit}");
    if all {
        url.push_str("&all=true");
    }

    let resp = reqwest::Client::new()
        .get(&url)
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let sessions: Vec<TunnelSession> = resp.json().await?;

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

fn format_timestamp(ts: &str) -> &str {
    let without_frac = ts.split('.').next().unwrap_or(ts);
    without_frac.strip_suffix('Z').unwrap_or(without_frac)
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
