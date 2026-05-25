mod island;
mod model;
mod ui;

use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use bytes::Bytes;
use chrono::{DateTime, TimeDelta, Utc};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use funnel_core::protocol::request::{HttpRequest, HttpResponse};

use self::island::IslandExchange;
pub use self::model::{BodyPreview, append_preview, body_preview};
use self::model::{CapturedExchange, ExchangeSource};

const MAX_ENTRIES: usize = 250;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Inspector {
    state: Arc<InspectorState>,
}

#[derive(Clone)]
pub struct InspectorHandle {
    state: Arc<InspectorState>,
}

struct InspectorState {
    local_addr: String,
    public_url: String,
    tunnel_id: String,
    started: Instant,
    started_at: DateTime<Utc>,
    entries: Mutex<VecDeque<CapturedExchange>>,
    events: broadcast::Sender<CapturedExchange>,
}

#[derive(Debug, Serialize)]
struct InspectorSnapshot {
    tunnel_id: String,
    public_url: String,
    local_addr: String,
    uptime_secs: u64,
    requests: Vec<CapturedExchange>,
}

#[derive(Debug, Deserialize)]
struct ReplayRequest {
    method: String,
    path: String,
    headers: String,
    body: String,
}

impl Inspector {
    pub fn new(local_addr: String, public_url: String, tunnel_id: String) -> Self {
        let (events, _) = broadcast::channel(MAX_ENTRIES);
        Self {
            state: Arc::new(InspectorState {
                local_addr,
                public_url,
                tunnel_id,
                started: Instant::now(),
                started_at: Utc::now(),
                entries: Mutex::new(VecDeque::new()),
                events,
            }),
        }
    }

    pub fn handle(&self) -> InspectorHandle {
        InspectorHandle {
            state: Arc::clone(&self.state),
        }
    }

    pub async fn serve(self, addr: SocketAddr, shutdown: CancellationToken) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let app = Router::new()
            .route("/", get(index))
            .route("/api/requests", get(requests))
            .route("/api/events", get(events))
            .route("/api/har", get(download_all_har))
            .route("/api/requests/{id}/har", get(download_request_har))
            .route("/api/replay/{id}", post(replay))
            .route("/requests/{id}/replay", post(replay_form))
            .with_state(self.state);

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown.cancelled().await;
            })
            .await?;

        Ok(())
    }
}

impl InspectorHandle {
    pub fn record_tunnel(
        &self,
        request: &HttpRequest,
        request_body: &Bytes,
        response: &HttpResponse,
        response_body: BodyPreview,
        duration: Duration,
    ) {
        self.push(CapturedExchange {
            id: Uuid::now_v7().to_string(),
            sequence: 0,
            source: ExchangeSource::Tunnel,
            timestamp_ms: self.state.started.elapsed().as_millis(),
            remote_addr: request.remote_addr.clone(),
            method: request.method.clone(),
            path: request.path.clone(),
            request_headers: request.headers.clone(),
            request_body: body_preview(request_body),
            status: response.status,
            response_headers: response.headers.clone(),
            response_body,
            duration_ms: duration.as_millis(),
        });
    }

    pub fn record_upgrade(
        &self,
        request: &HttpRequest,
        response: &HttpResponse,
        duration: Duration,
    ) {
        self.push(CapturedExchange {
            id: Uuid::now_v7().to_string(),
            sequence: 0,
            source: ExchangeSource::Tunnel,
            timestamp_ms: self.state.started.elapsed().as_millis(),
            remote_addr: request.remote_addr.clone(),
            method: request.method.clone(),
            path: request.path.clone(),
            request_headers: request.headers.clone(),
            request_body: BodyPreview::empty(),
            status: response.status,
            response_headers: response.headers.clone(),
            response_body: BodyPreview::empty(),
            duration_ms: duration.as_millis(),
        });
    }

    fn push(&self, mut exchange: CapturedExchange) {
        let mut entries = self
            .state
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        exchange.sequence = entries.back().map_or(1, |entry| entry.sequence + 1);
        entries.push_back(exchange.clone());
        while entries.len() > MAX_ENTRIES {
            entries.pop_front();
        }
        drop(entries);

        let _ = self.state.events.send(exchange);
    }
}

async fn requests(State(state): State<Arc<InspectorState>>) -> Json<InspectorSnapshot> {
    Json(snapshot(&state))
}

async fn download_all_har(State(state): State<Arc<InspectorState>>) -> impl IntoResponse {
    let entries = state
        .entries
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    har_response(
        "funnel-requests.har",
        serde_json::to_string_pretty(&har_log(&state, &entries)),
    )
}

async fn download_request_har(
    State(state): State<Arc<InspectorState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let exchange = state
        .entries
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|entry| entry.id == id)
        .cloned();

    let Some(exchange) = exchange else {
        return (StatusCode::NOT_FOUND, "request not found").into_response();
    };

    let filename = format!("funnel-request-{}.har", safe_filename(&exchange.id));
    har_response(
        &filename,
        serde_json::to_string_pretty(&har_log(&state, &[exchange])),
    )
}

#[derive(Debug, Deserialize)]
struct PageQuery {
    request: Option<String>,
    filter: Option<String>,
    q: Option<String>,
}

async fn index(
    State(state): State<Arc<InspectorState>>,
    Query(query): Query<PageQuery>,
) -> Html<String> {
    let snapshot = snapshot(&state);
    Html(ui::render_index(ui::PageData {
        tunnel_id: snapshot.tunnel_id,
        public_url: snapshot.public_url,
        local_addr: snapshot.local_addr,
        requests: snapshot.requests,
        selected_id: query.request,
        filter: query.filter.unwrap_or_else(|| "all".to_string()),
        query: query.q.unwrap_or_default(),
    }))
}

async fn events(
    State(state): State<Arc<InspectorState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(exchange) => return Some((Ok(exchange_event(&exchange)), rx)),
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn replay(
    State(state): State<Arc<InspectorState>>,
    Path(id): Path<String>,
    Json(payload): Json<ReplayRequest>,
) -> impl IntoResponse {
    let original = {
        let entries = state.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.iter().find(|entry| entry.id == id).cloned()
    };

    if original.is_none() {
        return (StatusCode::NOT_FOUND, "request not found").into_response();
    }

    match replay_request(&state, payload).await {
        Ok(exchange) => {
            let handle = InspectorHandle { state };
            handle.push(exchange.clone());
            Json(exchange).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

fn exchange_event(exchange: &CapturedExchange) -> Event {
    let payload = serde_json::json!({
        "exchange": island_exchange(exchange),
    });
    match serde_json::to_string(&payload) {
        Ok(json) => Event::default().event("exchange").data(json),
        Err(e) => Event::default()
            .event("error")
            .data(format!("failed to serialize exchange: {e}")),
    }
}

fn har_response(filename: &str, body: serde_json::Result<String>) -> axum::response::Response {
    match body {
        Ok(body) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (
                    header::CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"{filename}\""),
                ),
            ],
            body,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

fn safe_filename(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn har_log(state: &InspectorState, entries: &[CapturedExchange]) -> serde_json::Value {
    serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {
                "name": "Funnel Inspector",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "entries": entries
                .iter()
                .map(|entry| har_entry(state, entry))
                .collect::<Vec<_>>(),
        }
    })
}

fn har_entry(state: &InspectorState, exchange: &CapturedExchange) -> serde_json::Value {
    serde_json::json!({
        "startedDateTime": har_started_date_time(state, exchange),
        "time": exchange.duration_ms,
        "request": har_request(state, exchange),
        "response": har_response_entry(exchange),
        "cache": {},
        "timings": {
            "send": 0,
            "wait": exchange.duration_ms,
            "receive": 0,
        },
    })
}

fn har_request(state: &InspectorState, exchange: &CapturedExchange) -> serde_json::Value {
    let mut request = serde_json::json!({
        "method": exchange.method,
        "url": exchange_url(&state.public_url, &exchange.path),
        "httpVersion": "HTTP/1.1",
        "cookies": [],
        "headers": har_headers(&exchange.request_headers),
        "queryString": har_query_string(&exchange.path),
        "headersSize": -1,
        "bodySize": exchange.request_body.bytes,
    });

    if exchange.request_body.bytes > 0 {
        request["postData"] = har_post_data(&exchange.request_body, &exchange.request_headers);
    }

    request
}

fn har_response_entry(exchange: &CapturedExchange) -> serde_json::Value {
    serde_json::json!({
        "status": exchange.status,
        "statusText": "",
        "httpVersion": "HTTP/1.1",
        "cookies": [],
        "headers": har_headers(&exchange.response_headers),
        "content": har_content(&exchange.response_body, &exchange.response_headers),
        "redirectURL": redirect_url(&exchange.response_headers),
        "headersSize": -1,
        "bodySize": exchange.response_body.bytes,
    })
}

fn har_post_data(body: &BodyPreview, headers: &HashMap<String, Vec<String>>) -> serde_json::Value {
    let mut post_data = serde_json::json!({
        "mimeType": content_type(headers).unwrap_or_else(|| "application/octet-stream".to_string()),
        "text": har_body_text(body),
    });
    if body.binary {
        post_data["encoding"] = serde_json::Value::String("base64".to_string());
    }
    if body.truncated {
        post_data["comment"] = serde_json::Value::String(format!(
            "Body preview truncated to {} bytes.",
            body.preview_bytes.len()
        ));
    }
    post_data
}

fn har_content(body: &BodyPreview, headers: &HashMap<String, Vec<String>>) -> serde_json::Value {
    let mut content = serde_json::json!({
        "size": body.bytes,
        "mimeType": content_type(headers).unwrap_or_else(|| "application/octet-stream".to_string()),
    });
    if body.bytes > 0 {
        content["text"] = serde_json::Value::String(har_body_text(body));
    }
    if body.binary {
        content["encoding"] = serde_json::Value::String("base64".to_string());
    }
    if body.truncated {
        content["comment"] = serde_json::Value::String(format!(
            "Body preview truncated to {} bytes.",
            body.preview_bytes.len()
        ));
    }
    content
}

fn har_body_text(body: &BodyPreview) -> String {
    if body.binary {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode(&body.preview_bytes)
    } else {
        body.text.clone()
    }
}

fn har_headers(headers: &HashMap<String, Vec<String>>) -> Vec<serde_json::Value> {
    headers
        .iter()
        .flat_map(|(name, values)| {
            values
                .iter()
                .map(move |value| serde_json::json!({ "name": name, "value": value }))
        })
        .collect()
}

fn har_query_string(path: &str) -> Vec<serde_json::Value> {
    path.split_once('?')
        .map(|(_, query)| {
            url::form_urlencoded::parse(query.as_bytes())
                .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
                .collect()
        })
        .unwrap_or_default()
}

fn har_started_date_time(state: &InspectorState, exchange: &CapturedExchange) -> String {
    let millis = i64::try_from(exchange.timestamp_ms).unwrap_or(i64::MAX);
    state
        .started_at
        .checked_add_signed(TimeDelta::milliseconds(millis))
        .unwrap_or(state.started_at)
        .to_rfc3339()
}

fn exchange_url(public_url: &str, path: &str) -> String {
    let Ok(mut url) = url::Url::parse(public_url) else {
        return format!(
            "{}{}",
            public_url.trim_end_matches('/'),
            normalized_path(path)
        );
    };
    let path = normalized_path(path);
    let (path_only, query) = path.split_once('?').unwrap_or((&path, ""));
    url.set_path(path_only);
    if query.is_empty() {
        url.set_query(None);
    } else {
        url.set_query(Some(query));
    }
    url.to_string()
}

fn normalized_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn content_type(headers: &HashMap<String, Vec<String>>) -> Option<String> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, values)| values.first())
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or(value)
                .trim()
                .to_lowercase()
        })
        .filter(|value| !value.is_empty())
}

fn redirect_url(headers: &HashMap<String, Vec<String>>) -> String {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("location"))
        .and_then(|(_, values)| values.first())
        .cloned()
        .unwrap_or_default()
}

fn island_exchange(exchange: &CapturedExchange) -> IslandExchange {
    let source = match exchange.source {
        ExchangeSource::Tunnel => "tunnel",
        ExchangeSource::Replay => "replay",
    };
    IslandExchange {
        id: exchange.id.clone(),
        method: exchange.method.clone(),
        path: exchange.path.clone(),
        status: exchange.status,
        source: source.to_string(),
        duration_ms: exchange.duration_ms,
    }
}

async fn replay_form(
    State(state): State<Arc<InspectorState>>,
    Path(id): Path<String>,
    Form(payload): Form<ReplayRequest>,
) -> impl IntoResponse {
    let original = {
        let entries = state.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.iter().find(|entry| entry.id == id).cloned()
    };

    if original.is_none() {
        return (StatusCode::NOT_FOUND, "request not found").into_response();
    }

    match replay_request(&state, payload).await {
        Ok(exchange) => {
            let id = exchange.id.clone();
            let handle = InspectorHandle { state };
            handle.push(exchange);
            Redirect::to(&format!("/?request={id}")).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

async fn replay_request(
    state: &InspectorState,
    payload: ReplayRequest,
) -> anyhow::Result<CapturedExchange> {
    let started = Instant::now();
    let method: reqwest::Method = payload.method.parse()?;
    let url = replay_url(&state.local_addr, &payload.path);
    let client = reqwest::Client::builder()
        .timeout(DEFAULT_TIMEOUT)
        .danger_accept_invalid_certs(true)
        .build()?;

    let parsed_headers = parse_headers(&payload.headers);
    let mut request = client.request(method, &url).body(payload.body.clone());
    for (name, values) in &parsed_headers {
        for value in values {
            request = request.header(name, value);
        }
    }

    let response = request.send().await?;
    let status = response.status().as_u16();
    let response_headers = response_headers(response.headers());
    let body = response.bytes().await?;
    let response_body = body_preview(&body);

    Ok(CapturedExchange {
        id: Uuid::now_v7().to_string(),
        sequence: 0,
        source: ExchangeSource::Replay,
        timestamp_ms: state.started.elapsed().as_millis(),
        remote_addr: "inspector".to_string(),
        method: payload.method,
        path: payload.path,
        request_headers: parsed_headers,
        request_body: body_preview(&Bytes::from(payload.body)),
        status,
        response_headers,
        response_body,
        duration_ms: started.elapsed().as_millis(),
    })
}

fn replay_url(local_addr: &str, path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("http://{local_addr}{path}")
}

fn parse_headers(raw: &str) -> HashMap<String, Vec<String>> {
    let mut headers = HashMap::new();
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim();
            if skip_replay_header(name) {
                continue;
            }
            headers
                .entry(name.to_string())
                .or_insert_with(Vec::new)
                .push(value.trim().to_string());
        }
    }
    headers
}

fn skip_replay_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "host"
            | "content-length"
            | "transfer-encoding"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "upgrade"
            | "proxy-connection"
            | "accept-encoding"
    )
}

fn response_headers(headers: &reqwest::header::HeaderMap) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers {
        out.entry(name.to_string())
            .or_default()
            .push(value.to_str().unwrap_or("").to_string());
    }
    out
}

fn snapshot(state: &InspectorState) -> InspectorSnapshot {
    let entries = state
        .entries
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .rev()
        .cloned()
        .collect();

    InspectorSnapshot {
        tunnel_id: state.tunnel_id.clone(),
        public_url: state.public_url.clone(),
        local_addr: state.local_addr.clone(),
        uptime_secs: state.started.elapsed().as_secs(),
        requests: entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn har_log_contains_request_response_and_query() {
        let inspector = Inspector::new(
            "127.0.0.1:3000".to_string(),
            "https://example.com".to_string(),
            "tid_test".to_string(),
        );
        let mut request_headers = HashMap::new();
        request_headers.insert(
            "content-type".to_string(),
            vec!["application/json".to_string()],
        );
        let mut response_headers = HashMap::new();
        response_headers.insert("location".to_string(), vec!["/next".to_string()]);

        let exchange = CapturedExchange {
            id: "req/one".to_string(),
            sequence: 1,
            source: ExchangeSource::Tunnel,
            timestamp_ms: 12,
            remote_addr: "203.0.113.10:443".to_string(),
            method: "POST".to_string(),
            path: "/submit?debug=true".to_string(),
            request_headers,
            request_body: body_preview(&Bytes::from_static(br#"{"ok":true}"#)),
            status: 302,
            response_headers,
            response_body: BodyPreview::empty(),
            duration_ms: 37,
        };

        let har = har_log(&inspector.state, &[exchange]);
        let entry = &har["log"]["entries"][0];

        assert_eq!(entry["request"]["method"], "POST");
        assert_eq!(
            entry["request"]["url"],
            "https://example.com/submit?debug=true"
        );
        assert_eq!(entry["request"]["queryString"][0]["name"], "debug");
        assert_eq!(entry["request"]["postData"]["mimeType"], "application/json");
        assert_eq!(entry["response"]["status"], 302);
        assert_eq!(entry["response"]["redirectURL"], "/next");
        assert_eq!(entry["timings"]["wait"], 37);
    }

    #[test]
    fn binary_har_body_uses_base64_preview() {
        let body = body_preview(&Bytes::from_static(b"\0font"));
        let content = har_content(&body, &HashMap::new());

        assert_eq!(content["encoding"], "base64");
        assert_eq!(content["text"], "AGZvbnQ=");
    }
}
