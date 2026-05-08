use axum::Router;
use axum::extract::Request;
use axum::response::Redirect;

pub fn router(tls_port: u16) -> Router {
    Router::new().fallback(move |request: Request| async move {
        let host = request
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        let host_without_port = host.split(':').next().unwrap_or(host);
        let path = request
            .uri()
            .path_and_query()
            .map_or("/", axum::http::uri::PathAndQuery::as_str);

        let location = if tls_port == 443 {
            format!("https://{host_without_port}{path}")
        } else {
            format!("https://{host_without_port}:{tls_port}{path}")
        };
        Redirect::permanent(&location)
    })
}
