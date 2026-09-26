use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use proto::users::{CreateUserRequest, GetUserRequest};
use serde::{Deserialize, Serialize};

use crate::{AppState, errors::grpc_error};

#[derive(Debug, Deserialize)]
pub struct CreateUserBody {
    name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct UserResponse {
    id: String,
    name: String,
    email: String,
    created_at: Option<String>,
}

impl From<proto::users::User> for UserResponse {
    fn from(user: proto::users::User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            created_at: user
                .created_at
                .map(|timestamp| format!("{}.{:09}Z", timestamp.seconds, timestamp.nanos)),
        }
    }
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserBody>,
) -> Response {
    let mut client = state.users.clone();
    let response = client
        .create_user(CreateUserRequest {
            name: body.name,
            email: body.email,
        })
        .await;

    match response {
        Ok(response) => (
            StatusCode::CREATED,
            Json(UserResponse::from(response.into_inner())),
        )
            .into_response(),
        Err(status) => grpc_error(status),
    }
}

pub async fn get_user(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let mut client = state.users.clone();
    match client.get_user(GetUserRequest { id }).await {
        Ok(response) => Json(UserResponse::from(response.into_inner())).into_response(),
        Err(status) => grpc_error(status),
    }
}

pub async fn list_users(State(state): State<AppState>) -> Response {
    let mut client = state.users.clone();
    match client.list_users(proto::users::Empty {}).await {
        Ok(response) => {
            let mut stream = response.into_inner();
            let mut users = Vec::new();
            while let Some(user) = stream.next().await {
                match user {
                    Ok(user) => users.push(UserResponse::from(user)),
                    Err(status) => return grpc_error(status),
                }
            }
            Json(users).into_response()
        }
        Err(status) => grpc_error(status),
    }
}
