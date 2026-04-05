use std::collections::HashMap;

use funnel_core::protocol::{RequestPayload, ResponsePayload};

const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "proxy-connection",
];

pub struct Forwarder {
    client: reqwest::Client,
    local_addr: String,
}

impl Forwarder {
    pub fn new(local_addr: String) -> Self {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .expect("failed to build http client");

        Self { client, local_addr }
    }

    pub async fn forward(&self, req: RequestPayload) -> ResponsePayload {
        match self.try_forward(&req).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(error = %e, method = %req.method, path = %req.path, "forwarding failed");
                error_response(502, &format!("local service error: {e}"))
            }
        }
    }

    async fn try_forward(&self, req: &RequestPayload) -> anyhow::Result<ResponsePayload> {
        let url = format!("http://{}{}", self.local_addr, req.path);

        let method: http::Method = req.method.parse()?;
        let mut builder = self.client.request(method, &url);

        for (name, values) in &req.headers {
            if is_hop_by_hop(name) || name.eq_ignore_ascii_case("host") {
                continue;
            }
            for value in values {
                builder = builder.header(name.as_str(), value.as_str());
            }
        }

        builder = builder.header("host", &self.local_addr);
        builder = builder.body(req.body.clone());

        let resp = builder.send().await?;

        let status = resp.status().as_u16();
        let headers = collect_response_headers(resp.headers());
        let body = resp.bytes().await?.to_vec();

        Ok(ResponsePayload {
            status,
            headers,
            body,
        })
    }
}

fn is_hop_by_hop(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    HOP_BY_HOP_HEADERS.iter().any(|h| *h == lower)
}

fn collect_response_headers(headers: &reqwest::header::HeaderMap) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers.iter() {
        if is_hop_by_hop(name.as_str()) {
            continue;
        }
        let val = value.to_str().unwrap_or("").to_string();
        map.entry(name.to_string()).or_default().push(val);
    }
    map
}

fn error_response(status: u16, msg: &str) -> ResponsePayload {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), vec!["text/plain".to_string()]);
    ResponsePayload {
        status,
        headers,
        body: msg.as_bytes().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hop_by_hop_detection() {
        assert!(is_hop_by_hop("Connection"));
        assert!(is_hop_by_hop("transfer-encoding"));
        assert!(is_hop_by_hop("Keep-Alive"));
        assert!(!is_hop_by_hop("Content-Type"));
        assert!(!is_hop_by_hop("Authorization"));
    }

    #[test]
    fn error_response_has_correct_status() {
        let resp = error_response(502, "test error");
        assert_eq!(resp.status, 502);
        assert_eq!(resp.body, b"test error");
        assert!(resp.headers.contains_key("content-type"));
    }
}
