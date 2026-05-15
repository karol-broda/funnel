use funnel_core::api::Endpoint;
use funnel_core::protocol::PROTOCOL_VERSION;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct ApiClient {
    base_url: String,
    http: reqwest::Client,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(server_url: &str, token: Option<String>) -> Self {
        Self {
            base_url: server_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
            token,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v{PROTOCOL_VERSION}{path}", self.base_url)
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.token {
            Some(ref token) => builder.header("authorization", format!("Bearer {token}")),
            None => builder,
        }
    }

    pub async fn request<Resp: DeserializeOwned>(
        &self,
        endpoint: &Endpoint<(), Resp>,
    ) -> anyhow::Result<Resp> {
        let url = self.url(endpoint.path);
        let builder = self.http.request(endpoint.method.clone(), &url);
        let resp = self.apply_auth(builder).send().await?;
        handle_response(resp).await
    }

    #[allow(dead_code)]
    pub async fn request_with<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        endpoint: &Endpoint<Req, Resp>,
        body: &Req,
    ) -> anyhow::Result<Resp> {
        let url = self.url(endpoint.path);
        let builder = self.http.request(endpoint.method.clone(), &url).json(body);
        let resp = self.apply_auth(builder).send().await?;
        handle_response(resp).await
    }
}

async fn handle_response<Resp: DeserializeOwned>(resp: reqwest::Response) -> anyhow::Result<Resp> {
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }
    let value: serde_json::Value = resp.json().await?;
    let inner = unwrap_envelope(value);
    Ok(serde_json::from_value(inner)?)
}

fn unwrap_envelope(value: serde_json::Value) -> serde_json::Value {
    if value.get("kind").is_some() && value.get("data").is_some() {
        value["data"].clone()
    } else {
        value
    }
}
