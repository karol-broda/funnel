use serde::{Deserialize, Serialize};

/// associates a serializable type with its envelope kind name.
pub trait Enveloped: Serialize {
    const KIND: &'static str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub kind: String,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<EnvelopeMeta>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvelopeMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ErrorData {
    #[serde(rename = "type")]
    pub error_type: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub status: u16,
}

impl<T: Enveloped> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            kind: T::KIND.into(),
            data,
            meta: None,
        }
    }
}

impl<T: Enveloped> Envelope<Vec<T>> {
    pub fn list(data: Vec<T>) -> Self {
        let total = data.len() as u64;
        Self {
            kind: format!("{}_list", T::KIND),
            data,
            meta: Some(EnvelopeMeta {
                total: Some(total),
                ..Default::default()
            }),
        }
    }
}

impl Envelope<ErrorData> {
    pub fn error(
        error_type: impl std::fmt::Display,
        title: impl Into<String>,
        detail: Option<String>,
        status: u16,
    ) -> Self {
        Self {
            kind: "error".into(),
            data: ErrorData {
                error_type: error_type.to_string(),
                title: title.into(),
                detail,
                status,
            },
            meta: None,
        }
    }
}

impl<T> Envelope<T> {
    #[must_use]
    pub fn with_meta(mut self, meta: EnvelopeMeta) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Enveloped for serde_json::Value {
    const KIND: &'static str = "result";
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize)]
    struct TestItem {
        id: String,
    }

    impl Enveloped for TestItem {
        const KIND: &'static str = "test_item";
    }

    #[test]
    fn ok_infers_kind_from_type() {
        let item = TestItem { id: "abc".into() };
        let envelope = Envelope::ok(item);
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["kind"], "test_item");
        assert_eq!(json["data"]["id"], "abc");
        assert!(json.get("meta").is_none());
    }

    #[test]
    fn list_infers_kind_and_total() {
        let items = vec![TestItem { id: "a".into() }, TestItem { id: "b".into() }];
        let envelope = Envelope::list(items);
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["kind"], "test_item_list");
        assert_eq!(json["meta"]["total"], 2);
    }

    #[test]
    fn error_without_detail() {
        let envelope = Envelope::error("not_found", "not found", None, 404);
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["kind"], "error");
        assert_eq!(json["data"]["type"], "not_found");
        assert_eq!(json["data"]["status"], 404);
        assert!(json["data"].get("detail").is_none());
    }

    #[test]
    fn error_with_detail() {
        let envelope = Envelope::error(
            "tunnel_id_conflict",
            "conflict",
            Some("tunnel 'foo' already in use".into()),
            409,
        );
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["data"]["detail"], "tunnel 'foo' already in use");
    }

    #[test]
    fn error_accepts_app_code_enum() {
        use crate::protocol::error_codes::AppCode;

        let envelope = Envelope::error(AppCode::AuthRequired, "unauthorized", None, 401);
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["data"]["type"], "auth_required");
        assert_eq!(json["data"]["status"], 401);
    }

    #[test]
    fn error_app_code_with_detail() {
        use crate::protocol::error_codes::AppCode;

        let envelope = Envelope::error(
            AppCode::TunnelIdConflict,
            "conflict",
            Some("tunnel 'abc' in use".into()),
            409,
        );
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["data"]["type"], "tunnel_id_conflict");
        assert_eq!(json["data"]["detail"], "tunnel 'abc' in use");
    }

    #[test]
    fn empty_list_has_zero_total() {
        let items: Vec<TestItem> = vec![];
        let envelope = Envelope::list(items);
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["kind"], "test_item_list");
        assert_eq!(json["meta"]["total"], 0);
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn with_meta_adds_metadata() {
        let item = TestItem { id: "x".into() };
        let envelope = Envelope::ok(item).with_meta(EnvelopeMeta {
            request_id: Some("req-123".into()),
            timestamp: Some("2025-01-01T00:00:00Z".into()),
            total: None,
            cursor: None,
        });
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["meta"]["request_id"], "req-123");
        assert_eq!(json["meta"]["timestamp"], "2025-01-01T00:00:00Z");
        assert!(json["meta"].get("total").is_none());
        assert!(json["meta"].get("cursor").is_none());
    }
}
