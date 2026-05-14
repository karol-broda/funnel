use axum::Json;
use axum::response::{IntoResponse, Response};
use funnel_core::api::Enveloped;
use funnel_core::api::envelope::Envelope;

/// envelope response for a single item.
pub struct One<T: Enveloped>(pub T);

/// envelope response for a list of items, includes total count in meta.
pub struct Many<T: Enveloped>(pub Vec<T>);

impl<T: Enveloped> IntoResponse for One<T> {
    fn into_response(self) -> Response {
        Json(Envelope::ok(self.0)).into_response()
    }
}

impl<T: Enveloped> IntoResponse for Many<T> {
    fn into_response(self) -> Response {
        Json(Envelope::list(self.0)).into_response()
    }
}
