use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandExchange {
    pub id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub source: String,
    pub duration_ms: u128,
}

#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn RequestList(
    entries: Vec<IslandExchange>,
    filter: String,
    query: String,
    selected_id: Option<String>,
) -> impl IntoView {
    let filter_for_rows = filter.clone();
    let query_for_rows = query.clone();

    view! {
        <div
            id="request-list"
            class="request-list"
            data-filter=filter
            data-query=query
        >
            {entries
                .into_iter()
                .filter(|exchange| matches_filter(exchange, &filter_for_rows, &query_for_rows))
                .map(|exchange| {
                    let selected = selected_id.as_deref() == Some(exchange.id.as_str());
                    view! {
                        <RequestRow
                            exchange=exchange
                            selected=selected
                            filter=filter_for_rows.clone()
                            query=query_for_rows.clone()
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn RequestRow(
    exchange: IslandExchange,
    selected: bool,
    filter: String,
    query: String,
) -> impl IntoView {
    let class = if selected {
        "request-row active"
    } else {
        "request-row"
    };
    let status_class = status_class(exchange.status);
    let href = request_href(&exchange.id, &filter, &query);

    view! {
        <a
            class=class
            href=href
            data-id=exchange.id.clone()
            data-method=exchange.method.clone()
            data-path=exchange.path.clone()
            data-status=exchange.status.to_string()
            data-source=exchange.source
        >
            <span class="method-pill">{exchange.method.clone()}</span>
            <span class="request-main">
                <span class="path" title=exchange.path.clone()>{exchange.path.clone()}</span>
                <span class="sub">{exchange.source.clone()}" - "{exchange.duration_ms}"ms"</span>
            </span>
            <span class=format!("status {status_class}")>{exchange.status}</span>
        </a>
    }
}

fn request_href(id: &str, filter: &str, query: &str) -> String {
    let mut href = format!("/?request={}", url_escape(id));
    if !filter.is_empty() && filter != "all" {
        href.push_str("&filter=");
        href.push_str(&url_escape(filter));
    }
    if !query.is_empty() {
        href.push_str("&q=");
        href.push_str(&url_escape(query));
    }
    href
}

pub fn matches_filter(entry: &IslandExchange, filter: &str, query: &str) -> bool {
    let status_matches = match filter {
        "2xx" => (200..300).contains(&entry.status),
        "4xx" => (400..500).contains(&entry.status),
        "5xx" => entry.status >= 500,
        "replay" => entry.source == "replay",
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

pub const fn status_class(status: u16) -> &'static str {
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
