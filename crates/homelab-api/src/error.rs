use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use homelab_core::HomelabError;
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> axum::Json<Self> {
        axum::Json(Self {
            success: true,
            data: Some(data),
            error: None,
        })
    }
}

impl ApiResponse<()> {
    pub fn ok_empty() -> axum::Json<Self> {
        axum::Json(Self {
            success: true,
            data: None,
            error: None,
        })
    }
}

pub struct ApiError(pub HomelabError);

impl From<HomelabError> for ApiError {
    fn from(err: HomelabError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            HomelabError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            HomelabError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg.clone()),
            HomelabError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            HomelabError::Docker(_)
            | HomelabError::Cloudflare(_)
            | HomelabError::Database(_)
            | HomelabError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "an internal error occurred".into())
            }
        };

        tracing::error!(error = %self.0, "api error");

        let body = axum::Json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(message),
        });

        (status, body).into_response()
    }
}
