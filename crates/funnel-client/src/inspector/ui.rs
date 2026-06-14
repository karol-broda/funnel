use leptos::prelude::*;

use super::island::{IslandExchange, RequestList};
use super::model::{BodyPreview, CapturedExchange};

pub struct PageData {
    pub tunnel_id: String,
    pub public_url: String,
    pub local_addr: String,
    pub requests: Vec<CapturedExchange>,
    pub selected_id: Option<String>,
    pub filter: String,
    pub query: String,
}

pub fn render_index(data: PageData) -> String {
    let app = view! { <InspectorApp data=data /> }.to_html();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Funnel Inspector</title>
<style>{STYLE}</style>
<script>{THEME_BOOTSTRAP}</script>
</head>
<body>{app}<script>{LIVE_STREAM_SCRIPT}</script></body>
</html>"#
    )
}

#[component]
fn InspectorApp(data: PageData) -> impl IntoView {
    let filtered = filtered_requests(&data);
    let selected = data
        .selected_id
        .as_deref()
        .and_then(|id| data.requests.iter().find(|entry| entry.id == id))
        .cloned()
        .or_else(|| filtered.first().cloned());
    let selected_id = selected.as_ref().map(|entry| entry.id.clone());

    view! {
        <main id="app" class="app-shell">
            <InspectorRail
                tunnel_id=data.tunnel_id
                public_url=data.public_url
                local_addr=data.local_addr
            />
            <RequestColumn
                filter=data.filter
                query=data.query
                captured=filtered
                selected_id=selected_id
            />
            <DetailColumn selected=selected />
        </main>
    }
}

#[component]
fn InspectorRail(tunnel_id: String, public_url: String, local_addr: String) -> impl IntoView {
    view! {
        <aside class="rail">
            <div class="brand">
                <LogoMark />
                <div>
                    <p class="eyebrow">"client inspector"</p>
                    <h1>"Funnel"</h1>
                </div>
                <button class="theme-toggle" type="button" aria-label="Toggle color theme" title="Toggle theme">
                    <SunIcon />
                    <MoonIcon />
                </button>
            </div>

            <section class="facts" aria-label="Tunnel facts">
                <Fact label="Tunnel" value=tunnel_id />
                <LinkedFact label="Public URL" value=public_url.clone() href=public_url />
                <LinkedFact label="Forwarding" value=local_addr.clone() href=format!("http://{local_addr}") />
            </section>

            <section class="rail-actions" aria-label="Exports">
                <p class="eyebrow">"export"</p>
                <a class="btn subtle" href="/api/har" download="funnel-requests.har">"All HAR"</a>
            </section>
        </aside>
    }
}

#[component]
fn LogoMark() -> impl IntoView {
    view! {
        <svg class="logo-mark" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M4 4h16l-5.5 12h-5L4 4z M18.625 7L22 4.5" stroke="currentColor" stroke-width="1.5" stroke-linejoin="miter" fill="none"/>
            <path d="M12 16v5" stroke="currentColor" stroke-width="1.5"/>
        </svg>
    }
}

#[component]
fn SunIcon() -> impl IntoView {
    view! {
        <svg class="theme-icon sun" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor" aria-hidden="true">
            <path d="M120 40V16a8 8 0 0 1 16 0v24a8 8 0 0 1-16 0Zm8 24a64 64 0 1 0 64 64 64.07 64.07 0 0 0-64-64Zm0 112a48 48 0 1 1 48-48 48.05 48.05 0 0 1-48 48ZM58.34 69.66a8 8 0 0 0 11.32-11.32l-16-16a8 8 0 0 0-11.32 11.32Zm0 116.68-16 16a8 8 0 0 0 11.32 11.32l16-16a8 8 0 0 0-11.32-11.32ZM192 72a8 8 0 0 0 5.66-2.34l16-16a8 8 0 0 0-11.32-11.32l-16 16A8 8 0 0 0 192 72Zm5.66 114.34a8 8 0 0 0-11.32 11.32l16 16a8 8 0 0 0 11.32-11.32ZM48 128a8 8 0 0 0-8-8H16a8 8 0 0 0 0 16h24a8 8 0 0 0 8-8Zm80 80a8 8 0 0 0-8 8v24a8 8 0 0 0 16 0v-24a8 8 0 0 0-8-8Zm112-88h-24a8 8 0 0 0 0 16h24a8 8 0 0 0 0-16Z"/>
        </svg>
    }
}

#[component]
fn MoonIcon() -> impl IntoView {
    view! {
        <svg class="theme-icon moon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor" aria-hidden="true">
            <path d="M233.54 142.23a8 8 0 0 0-8-2 88.08 88.08 0 0 1-109.8-109.8 8 8 0 0 0-10-10 104.84 104.84 0 0 0-52.91 37A104 104 0 0 0 136 224a103.09 103.09 0 0 0 62.52-20.88 104.84 104.84 0 0 0 37-52.91 8 8 0 0 0-1.98-7.98Zm-44.64 48.11A88 88 0 0 1 65.66 67.11a89 89 0 0 1 31.4-26 106 106 0 0 0 117.83 117.83 89 89 0 0 1-25.99 31.4Z"/>
        </svg>
    }
}

#[component]
fn Fact(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="fact">
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[component]
fn LinkedFact(label: &'static str, value: String, href: String) -> impl IntoView {
    view! {
        <a class="fact fact-link" href=href target="_blank" rel="noreferrer">
            <span>{label}</span>
            <strong>{value}</strong>
        </a>
    }
}

#[component]
fn Filter(label: &'static str, filter: &'static str, active: bool, query: String) -> impl IntoView {
    let class = if active { "filter active" } else { "filter" };
    let href = if query.is_empty() {
        format!("/?filter={filter}")
    } else {
        format!("/?filter={filter}&q={}", url_escape(&query))
    };
    view! { <a class=class href=href>{label}</a> }
}

#[component]
fn RequestColumn(
    filter: String,
    query: String,
    captured: Vec<CapturedExchange>,
    selected_id: Option<String>,
) -> impl IntoView {
    let entries = captured
        .into_iter()
        .map(island_exchange)
        .collect::<Vec<IslandExchange>>();

    view! {
        <section class="request-column">
            <header class="column-header">
                <div class="column-title">
                    <div>
                        <p class="eyebrow">"traffic"</p>
                        <h2>"Requests"</h2>
                    </div>
                        <nav class="filters" aria-label="Request filters">
                            <Filter label="All" filter="all" active=filter == "all" query=query.clone() />
                            <Filter label="2xx" filter="2xx" active=filter == "2xx" query=query.clone() />
                            <Filter label="4xx" filter="4xx" active=filter == "4xx" query=query.clone() />
                            <Filter label="5xx" filter="5xx" active=filter == "5xx" query=query.clone() />
                        <Filter label="Replay" filter="replay" active=filter == "replay" query=query.clone() />
                    </nav>
                </div>
                <form class="search-form" method="get" action="/">
                    <input type="hidden" name="filter" value=filter.clone() />
                    <input id="search" type="search" class="input" name="q" value=query.clone() placeholder="Filter method, path, status" />
                    </form>
            </header>
            <RequestList entries=entries filter=filter query=query selected_id=selected_id />
        </section>
    }
}

#[component]
fn DetailColumn(selected: Option<CapturedExchange>) -> impl IntoView {
    view! {
        <section class="detail-column">
            {match selected {
                Some(exchange) => view! { <ExchangeDetail exchange=exchange /> }.into_any(),
                None => view! { <EmptyDetail /> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn EmptyDetail() -> impl IntoView {
    view! {
        <div class="empty-detail">
            <p>"Waiting for traffic"</p>
            <span>"Captured HTTP requests will appear here with headers, bodies, timings, and replay controls."</span>
        </div>
    }
}

#[component]
fn ExchangeDetail(exchange: CapturedExchange) -> impl IntoView {
    let headers = headers_to_text(&exchange.request_headers);
    let action = format!("/requests/{}/replay", exchange.id);
    let api_action = format!("/api/replay/{}", exchange.id);
    let har_action = format!("/api/requests/{}/har", exchange.id);
    let request_type = content_type(&exchange.request_headers);
    let response_type = content_type(&exchange.response_headers);
    let request_encoding = content_encoding(&exchange.request_headers);
    let response_encoding = content_encoding(&exchange.response_headers);

    view! {
        <article class="exchange-detail">
            <header class="detail-head">
                <div class="detail-title-block">
                        <p class="eyebrow">{format!("{:?}", exchange.source).to_lowercase()}</p>
                        <h2>{exchange.method.clone()}" "{exchange.path.clone()}</h2>
                </div>
                <div class="detail-actions">
                    <a class="btn subtle" href=har_action download title="Download request as HAR">"HAR"</a>
                    <form class="quick-replay replay-form" method="post" action=action.clone() data-api-action=api_action.clone()>
                        <input type="hidden" name="method" value=exchange.method.clone() />
                        <input type="hidden" name="path" value=exchange.path.clone() />
                            <textarea name="headers" style="display: none;">{headers.clone()}</textarea>
                            <textarea name="body" style="display: none;">{exchange.request_body.text.clone()}</textarea>
                            <button class="btn primary" type="submit">"Resend"</button>
                        </form>
                    </div>
                </header>

            <section class="summary">
                <Metric label="Status" value=exchange.status.to_string() class=status_class(exchange.status).to_string() />
                <Metric label="Duration" value=format!("{}ms", exchange.duration_ms) class=String::new() />
                <Metric label="Remote" value=exchange.remote_addr.clone() class=String::new() />
                <Metric
                    label="Request bytes"
                    value=body_size(&exchange.request_body)
                    class=String::new()
                />
                <Metric
                    label="Response bytes"
                    value=body_size(&exchange.response_body)
                    class=String::new()
                />
            </section>

            <div class="exchange-grid">
                <PayloadPanel
                    title="Request"
                    meta=body_size(&exchange.request_body)
                    headers=exchange.request_headers.clone()
                    body=exchange.request_body.clone()
                    content_type=request_type
                    content_encoding=request_encoding
                />
                <PayloadPanel
                    title="Response"
                    meta=body_size(&exchange.response_body)
                    headers=exchange.response_headers.clone()
                    body=exchange.response_body.clone()
                    content_type=response_type
                    content_encoding=response_encoding
                />
            </div>

            <details class="editor">
                <summary>"Replay composer"</summary>
                <form class="replay-form" method="post" action=action data-api-action=api_action>
                    <div class="editor-row">
                        <input class="input method" name="method" value=exchange.method.clone() />
                        <input class="input path-input" name="path" value=exchange.path.clone() />
                    </div>
                    <label class="field-label">
                        <span>"Headers"</span>
                        <HeaderEditor headers=exchange.request_headers.clone() raw=headers />
                    </label>
                    <label class="field-label">
                        <span>"Body"</span>
                        <div class="editor-tools">
                            <button class="btn subtle format-json" type="button">"Format JSON"</button>
                        </div>
                        <textarea class="textarea body body-editor" name="body" spellcheck="false">{exchange.request_body.text}</textarea>
                    </label>
                    <div class="form-actions">
                        <button class="btn primary" type="submit">"Send request"</button>
                        <span class="replay-status" aria-live="polite"></span>
                    </div>
                </form>
            </details>
        </article>
    }
}

#[component]
fn HeaderEditor(
    headers: std::collections::HashMap<String, Vec<String>>,
    raw: String,
) -> impl IntoView {
    let rows = header_rows(&headers);
    view! {
        <div class="headers-editor-table" data-header-editor>
            <textarea class="headers-editor" name="headers" spellcheck="false" hidden>{raw}</textarea>
            <div class="headers-editor-head" aria-hidden="true">
                <span>"Name"</span>
                <span>"Value"</span>
                <span></span>
            </div>
            <div class="headers-editor-rows">
                {rows
                    .into_iter()
                    .map(|(name, value)| view! { <HeaderEditorRow name=name value=value /> })
                    .collect_view()}
            </div>
            <button class="btn subtle add-header" type="button">"Add header"</button>
        </div>
    }
}

#[component]
fn HeaderEditorRow(name: String, value: String) -> impl IntoView {
    view! {
        <div class="headers-editor-row">
            <input class="input header-name" value=name placeholder="content-type" />
            <input class="input header-value" value=value placeholder="application/json" />
            <button class="btn subtle remove-header" type="button" aria-label="Remove header">"Remove"</button>
        </div>
    }
}

#[component]
fn PayloadPanel(
    title: &'static str,
    meta: String,
    headers: std::collections::HashMap<String, Vec<String>>,
    body: BodyPreview,
    content_type: Option<String>,
    content_encoding: Option<String>,
) -> impl IntoView {
    view! {
        <section class="payload-panel">
            <header class="payload-head">
                <h3>{title}</h3>
                <span>{content_label(&meta, content_type.as_deref())}</span>
            </header>
            <details open>
                <summary>"Headers"</summary>
                <HeaderTable headers=headers />
            </details>
            <details open>
                <summary>"Body"</summary>
                <BodyViewer body=body content_type=content_type content_encoding=content_encoding />
            </details>
        </section>
    }
}

#[component]
fn HeaderTable(headers: std::collections::HashMap<String, Vec<String>>) -> impl IntoView {
    let rows = header_rows(&headers);
    if rows.is_empty() {
        return view! { <p class="empty-inline">"(no headers)"</p> }.into_any();
    }

    view! {
        <div class="headers-table" role="table">
            {rows
                .into_iter()
                .map(|(name, value)| {
                    view! {
                        <div class="header-row" role="row">
                            <span role="cell">{name}</span>
                            <code role="cell">{value}</code>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

#[component]
fn BodyViewer(
    body: BodyPreview,
    content_type: Option<String>,
    content_encoding: Option<String>,
) -> impl IntoView {
    if body.bytes == 0 {
        return view! { <p class="empty-inline">"(empty body)"</p> }.into_any();
    }

    if let Some(encoding) = compressed_encoding(content_encoding.as_deref()) {
        let text = body.text;
        return view! {
            <div class="body-stack">
                <p class="empty-inline">{format!("Compressed {encoding} body captured. New requests ask the local service for an uncompressed response.")}</p>
                <details>
                    <summary>"Raw"</summary>
                    <pre>{text}</pre>
                </details>
            </div>
        }
        .into_any();
    }

    if font_content_type(content_type.as_deref()) {
        return view! { <FontPreview body=body content_type=content_type /> }.into_any();
    }

    let text = empty_label(&body.text);

    if body.binary {
        return view! { <p class="empty-inline">{text}</p> }.into_any();
    }

    if let Some(json) = parse_json(&text, content_type.as_deref()) {
        return view! {
            <div class="body-stack">
                <details open>
                    <summary>"Raw"</summary>
                    <pre>{pretty_json(&json)}</pre>
                </details>
                <details>
                    <summary>"JSON tree"</summary>
                    <JsonNode value=json name="root".to_string() root=true />
                </details>
            </div>
        }
        .into_any();
    }

    if previewable_markup(&text, content_type.as_deref()) {
        return view! {
            <div class="body-stack">
                <details open>
                    <summary>"Raw"</summary>
                    <pre>{text.clone()}</pre>
                </details>
                <details>
                    <summary>"Preview"</summary>
                    <iframe class="markup-preview" sandbox="" srcdoc=text title="Body preview"></iframe>
                </details>
            </div>
        }
        .into_any();
    }

    view! { <pre>{text}</pre> }.into_any()
}

#[component]
fn FontPreview(body: BodyPreview, content_type: Option<String>) -> AnyView {
    let Some(data_url) = font_data_url(&body, content_type.as_deref()) else {
        return view! { <p class="empty-inline">{body.text}</p> }.into_any();
    };
    let family = format!("funnel-preview-{}", body.bytes);
    let style = format!(
        "@font-face {{ font-family: '{family}'; src: url('{data_url}'); font-display: swap; }}"
    );

    view! {
        <div class="body-stack">
            <style>{style}</style>
            <div class="font-preview" style=format!("font-family: '{family}', sans-serif")>
                <p>"The quick brown fox jumps over the lazy dog."</p>
                <p>"Sphinx of black quartz, judge my vow."</p>
                <p>"0123456789 Aa Bb Cc Dd Ee Ff"</p>
            </div>
            <details>
                <summary>"Raw"</summary>
                <p class="empty-inline">{body.text}</p>
            </details>
        </div>
    }
    .into_any()
}

#[component]
fn JsonNode(value: serde_json::Value, name: String, root: bool) -> impl IntoView {
    match value {
        serde_json::Value::Object(map) => {
            let count = map.len();
            view! {
                <details class="json-node" open=root>
                    <summary>
                        <span class="json-key">{name}</span>
                        <span class="json-kind">"object · "{count}</span>
                    </summary>
                    <div class="json-children">
                        {map
                            .into_iter()
                            .map(|(key, value)| view! { <JsonNode value=value name=key root=false /> })
                            .collect_view()}
                    </div>
                </details>
            }
            .into_any()
        }
        serde_json::Value::Array(items) => {
            let count = items.len();
            view! {
                <details class="json-node" open=root>
                    <summary>
                        <span class="json-key">{name}</span>
                        <span class="json-kind">"array · "{count}</span>
                    </summary>
                    <div class="json-children">
                        {items
                            .into_iter()
                            .enumerate()
                            .map(|(index, value)| view! { <JsonNode value=value name=format!("[{index}]") root=false /> })
                            .collect_view()}
                    </div>
                </details>
            }
            .into_any()
        }
        scalar => view! {
            <div class="json-leaf">
                <span class="json-key">{name}</span>
                <code>{json_scalar(scalar)}</code>
            </div>
        }
        .into_any(),
    }
}

#[component]
fn Metric(label: &'static str, value: String, class: String) -> impl IntoView {
    view! {
        <div class="kv">
            <strong>{label}</strong>
            <span class=class>{value}</span>
        </div>
    }
}

fn filtered_requests(data: &PageData) -> Vec<CapturedExchange> {
    data.requests
        .iter()
        .filter(|entry| matches_filter(entry, &data.filter, &data.query))
        .cloned()
        .collect()
}

fn matches_filter(entry: &CapturedExchange, filter: &str, query: &str) -> bool {
    let status_matches = match filter {
        "2xx" => (200..300).contains(&entry.status),
        "4xx" => (400..500).contains(&entry.status),
        "5xx" => entry.status >= 500,
        "replay" => format!("{:?}", entry.source).eq_ignore_ascii_case("replay"),
        _ => true,
    };
    if !status_matches {
        return false;
    }
    if query.is_empty() {
        return true;
    }
    let haystack = format!("{} {} {}", entry.method, entry.path, entry.status).to_lowercase();
    haystack.contains(&query.to_lowercase())
}

fn island_exchange(exchange: CapturedExchange) -> IslandExchange {
    IslandExchange {
        id: exchange.id,
        method: exchange.method,
        path: exchange.path,
        status: exchange.status,
        source: format!("{:?}", exchange.source).to_lowercase(),
        duration_ms: exchange.duration_ms,
    }
}

fn headers_to_text(headers: &std::collections::HashMap<String, Vec<String>>) -> String {
    headers
        .iter()
        .flat_map(|(name, values)| values.iter().map(move |value| format!("{name}: {value}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn header_rows(headers: &std::collections::HashMap<String, Vec<String>>) -> Vec<(String, String)> {
    let mut rows = headers
        .iter()
        .flat_map(|(name, values)| {
            values
                .iter()
                .map(move |value| (name.clone(), value.clone()))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.0.to_lowercase());
    rows
}

fn content_type(headers: &std::collections::HashMap<String, Vec<String>>) -> Option<String> {
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

fn content_encoding(headers: &std::collections::HashMap<String, Vec<String>>) -> Option<String> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-encoding"))
        .and_then(|(_, values)| values.first())
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
}

fn compressed_encoding(encoding: Option<&str>) -> Option<&str> {
    encoding
        .map(str::trim)
        .filter(|value| !value.eq_ignore_ascii_case("identity"))
}

fn content_label(size: &str, content_type: Option<&str>) -> String {
    match content_type {
        Some(content_type) => format!("{size} · {content_type}"),
        None => size.to_string(),
    }
}

fn parse_json(body: &str, content_type: Option<&str>) -> Option<serde_json::Value> {
    let looks_json = content_type.is_some_and(|value| {
        value == "application/json" || value.ends_with("+json") || value.contains("/json")
    }) || body.trim_start().starts_with('{')
        || body.trim_start().starts_with('[');

    if looks_json {
        serde_json::from_str(body).ok()
    } else {
        None
    }
}

fn pretty_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn json_scalar(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value,
        other => other.to_string(),
    }
}

fn previewable_markup(body: &str, content_type: Option<&str>) -> bool {
    let trimmed = body.trim_start().to_lowercase();
    content_type.is_some_and(|value| {
        value == "text/html" || value == "application/xhtml+xml" || value == "image/svg+xml"
    }) || trimmed.starts_with("<!doctype html")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<svg")
}

fn font_content_type(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|value| {
        value.starts_with("font/")
            || value.contains("font")
            || matches!(
                value,
                "application/woff"
                    | "application/woff2"
                    | "application/x-font-ttf"
                    | "application/x-font-opentype"
                    | "application/vnd.ms-fontobject"
            )
    })
}

fn font_data_url(body: &BodyPreview, content_type: Option<&str>) -> Option<String> {
    if body.preview_bytes.is_empty() || body.preview_bytes.len() != body.bytes {
        return None;
    }
    let mime = content_type.unwrap_or("font/woff2");
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&body.preview_bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

fn body_size(body: &super::model::BodyPreview) -> String {
    if body.truncated {
        format!("{} (truncated)", body.bytes)
    } else {
        body.bytes.to_string()
    }
}

fn empty_label(value: &str) -> String {
    if value.is_empty() {
        "(empty body)".to_string()
    } else {
        value.to_string()
    }
}

const fn status_class(status: u16) -> &'static str {
    if status >= 500 {
        "s5"
    } else if status >= 400 {
        "s4"
    } else {
        "s2"
    }
}

fn url_escape(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

const LIVE_STREAM_SCRIPT: &str = include_str!("script.js");

const STYLE: &str = include_str!("style.css");

const THEME_BOOTSTRAP: &str = r#"
(() => {
  const stored = localStorage.getItem("funnel-inspector-theme");
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.dataset.theme = stored || (prefersDark ? "dark" : "light");
})();
"#;
