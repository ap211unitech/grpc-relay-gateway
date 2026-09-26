use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use proto::posts::{CreatePostRequest, DeletePostRequest, GetPostRequest, GetPostsForUserRequest};
use serde::{Deserialize, Serialize};

use crate::{AppState, errors::grpc_error};

#[derive(Debug, Deserialize)]
pub struct CreatePostBody {
    user_id: String,
    title: String,
    content: String,
}

#[derive(Serialize)]
struct PostResponse {
    id: String,
    user_id: String,
    title: String,
    content: String,
    created_at: Option<String>,
}

impl From<proto::posts::Post> for PostResponse {
    fn from(post: proto::posts::Post) -> Self {
        Self {
            id: post.id,
            user_id: post.user_id,
            title: post.title,
            content: post.content,
            created_at: post
                .created_at
                .map(|timestamp| format!("{}.{:09}Z", timestamp.seconds, timestamp.nanos)),
        }
    }
}

pub async fn create_post(
    State(state): State<AppState>,
    Json(body): Json<CreatePostBody>,
) -> Response {
    let mut client = state.posts.clone();
    match client
        .create_post(CreatePostRequest {
            user_id: body.user_id,
            title: body.title,
            content: body.content,
        })
        .await
    {
        Ok(response) => (
            StatusCode::CREATED,
            Json(PostResponse::from(response.into_inner())),
        )
            .into_response(),
        Err(status) => grpc_error(status),
    }
}

pub async fn get_post(State(state): State<AppState>, Path(post_id): Path<String>) -> Response {
    let mut client = state.posts.clone();
    match client.get_post(GetPostRequest { post_id }).await {
        Ok(response) => (
            StatusCode::OK,
            Json(PostResponse::from(response.into_inner())),
        )
            .into_response(),
        Err(status) => grpc_error(status),
    }
}

pub async fn list_posts_for_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Response {
    let mut client = state.posts.clone();
    match client
        .list_posts_for_user(GetPostsForUserRequest { user_id })
        .await
    {
        Ok(response) => (
            StatusCode::OK,
            Json(
                response
                    .into_inner()
                    .posts
                    .into_iter()
                    .map(PostResponse::from)
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(status) => grpc_error(status),
    }
}

#[derive(Deserialize)]
pub struct DeletePostBody {
    user_id: String,
    post_id: String,
}

pub async fn delete_post(
    State(state): State<AppState>,
    Json(body): Json<DeletePostBody>,
) -> Response {
    let mut client = state.posts.clone();
    match client
        .delete_post(DeletePostRequest {
            post_id: body.post_id,
            user_id: body.user_id,
        })
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(status) => grpc_error(status),
    }
}
