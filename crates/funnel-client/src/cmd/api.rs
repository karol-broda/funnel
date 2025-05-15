use funnel_core::api::Endpoint;
use funnel_core::protocol::PROTOCOL_VERSION;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub async fn call<Req, Resp: DeserializeOwned>(
    server: &str,
    token: &str,
    endpoint: &Endpoint<Req, Resp>,
    json: bool,
) -> anyhow::Result<Option<Resp>> {
    request(server, token, &endpoint.method, endpoint.path, json).await
}

pub async fn call_at<Req, Resp: DeserializeOwned>(
    server: &str,
    token: &str,
    endpoint: &Endpoint<Req, Resp>,
    path: &str,
    json: bool,
) -> anyhow::Result<Option<Resp>> {
    request(server, token, &endpoint.method, path, json).await
}

pub async fn send<Req: Serialize, Resp: DeserializeOwned>(
    server: &str,
    token: &str,
    endpoint: &Endpoint<Req, Resp>,
    body: &Req,
    json: bool,
) -> anyhow::Result<Option<Resp>> {
    request_body(server, token, &endpoint.method, endpoint.path, body, json).await
}

pub async fn send_at<Req: Serialize, Resp: DeserializeOwned>(
    server: &str,
    token: &str,
    endpoint: &Endpoint<Req, Resp>,
    path: &str,
    body: &Req,
    json: bool,
) -> anyhow::Result<Option<Resp>> {
    request_body(server, token, &endpoint.method, path, body, json).await
}

async fn request<T: DeserializeOwned>(
    server: &str,
    token: &str,
    method: &http::Method,
    path: &str,
    json: bool,
) -> anyhow::Result<Option<T>> {
    let url = format!("{server}/api/v{PROTOCOL_VERSION}{path}");
    let envelope = raw_send(reqwest::Client::new().request(method.clone(), &url), token).await?;
    if json {
        print_json(&envelope);
        return Ok(None);
    }
    Ok(Some(extract_data(envelope)?))
}

async fn request_body<B: Serialize, T: DeserializeOwned>(
    server: &str,
    token: &str,
    method: &http::Method,
    path: &str,
    body: &B,
    json: bool,
) -> anyhow::Result<Option<T>> {
    let url = format!("{server}/api/v{PROTOCOL_VERSION}{path}");
    let builder = reqwest::Client::new()
        .request(method.clone(), &url)
        .json(body);
    let envelope = raw_send(builder, token).await?;
    if json {
        print_json(&envelope);
        return Ok(None);
    }
    Ok(Some(extract_data(envelope)?))
}

async fn raw_send(
    builder: reqwest::RequestBuilder,
    token: &str,
) -> anyhow::Result<serde_json::Value> {
    let resp = builder
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    Ok(resp.json().await?)
}

fn extract_data<T: DeserializeOwned>(envelope: serde_json::Value) -> anyhow::Result<T> {
    let inner = match envelope.get("data") {
        Some(d) => d.clone(),
        None => envelope,
    };
    Ok(serde_json::from_value(inner)?)
}

fn print_json(envelope: &serde_json::Value) {
    println!("{}", serde_json::to_string(envelope).unwrap_or_default());
}
