use axum::http::StatusCode;
use axum::response::IntoResponse;
use metrics_exporter_prometheus::PrometheusHandle;

pub fn setup() -> anyhow::Result<PrometheusHandle> {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    Ok(builder.install_recorder()?)
}

pub async fn handler(
    axum::extract::State(handle): axum::extract::State<PrometheusHandle>,
) -> impl IntoResponse {
    let body = handle.render();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
