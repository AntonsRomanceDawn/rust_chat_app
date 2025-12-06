use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiErrorItem {
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl ApiErrorItem {
    pub fn new(code: &'static str, details: impl Into<Option<Value>>) -> Self {
        Self {
            code,
            details: details.into(),
        }
    }
}

#[derive(Debug)]
pub struct HttpError {
    status: StatusCode,
    body: Value,
}

impl HttpError {
    pub fn new(status: StatusCode, body: Value) -> Self {
        Self { status, body }
    }

    fn with_errors(status: StatusCode, errors: impl IntoIterator<Item = ApiErrorItem>) -> Self {
        Self::new(
            status,
            json!({
                "errors": errors.into_iter().collect::<Vec<_>>()
            }),
        )
    }

    pub fn bad_request(errors: impl IntoIterator<Item = ApiErrorItem>) -> Self {
        Self::with_errors(StatusCode::BAD_REQUEST, errors)
    }

    pub fn unauthorized(errors: impl IntoIterator<Item = ApiErrorItem>) -> Self {
        Self::with_errors(StatusCode::UNAUTHORIZED, errors)
    }

    pub fn internal(errors: impl IntoIterator<Item = ApiErrorItem>) -> Self {
        Self::with_errors(StatusCode::INTERNAL_SERVER_ERROR, errors)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self.body)).into_response()
    }
}
