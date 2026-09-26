use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    error: String,
}

impl ApiError {
    fn from_status(status: tonic::Status) -> Self {
        Self {
            error: status.message().to_string(),
        }
    }

    fn status_code(status: &tonic::Status) -> StatusCode {
        match status.code() {
            tonic::Code::InvalidArgument => StatusCode::BAD_REQUEST,
            tonic::Code::NotFound => StatusCode::NOT_FOUND,
            tonic::Code::PermissionDenied => StatusCode::FORBIDDEN,
            tonic::Code::FailedPrecondition => StatusCode::UNPROCESSABLE_ENTITY,
            tonic::Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub fn grpc_error(status: tonic::Status) -> Response {
    let code = ApiError::status_code(&status);
    (code, Json(ApiError::from_status(status))).into_response()
}
