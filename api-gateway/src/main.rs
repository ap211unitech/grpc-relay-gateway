use axum::{
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use proto::{
    posts::post_service_client::PostServiceClient, users::user_service_client::UserServiceClient,
};
use serde_json::json;
use tonic::transport::{Channel, Endpoint};

use crate::handlers::{
    posts::{create_post, delete_post, get_post, list_posts_for_user},
    users::{create_user, get_user, list_users},
};

mod errors;
mod handlers;

#[derive(Clone)]
struct AppState {
    users: UserServiceClient<Channel>,
    posts: PostServiceClient<Channel>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let state = AppState {
        users: UserServiceClient::new(
            Endpoint::from_static("http://127.0.0.1:50051").connect_lazy(),
        ),
        posts: PostServiceClient::new(
            Endpoint::from_static("http://127.0.0.1:50052").connect_lazy(),
        ),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", get(get_user))
        .route("/users/{id}/posts", get(list_posts_for_user))
        .route("/posts", post(create_post).delete(delete_post))
        .route("/posts/{id}", get(get_post))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;

    println!(
        "🚀 API Gateway running on: {}",
        listener.local_addr().unwrap()
    );

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    let response = json!({
        "message": "Server is healthy!"
    });

    (StatusCode::OK, Json(response))
}
