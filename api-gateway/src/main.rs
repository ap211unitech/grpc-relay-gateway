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
use std::{env, net::SocketAddr};
use tonic::transport::{Channel, Endpoint};

use crate::handlers::{
    posts::{create_post, delete_post, get_post, list_posts_for_user},
    users::{create_user, get_user, list_users},
};

mod errors;
mod handlers;

#[derive(Clone, Debug)]
struct AppState {
    users: UserServiceClient<Channel>,
    posts: PostServiceClient<Channel>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv()?;
    let user_service_url = format!("http://{}", env::var("USER_SERVICE_URL")?);
    let post_service_url = format!("http://{}", env::var("POST_SERVICE_URL")?);
    let gateway_url = env::var("API_GATEWAY_URL")?;
    let gateway_addr = gateway_url.parse::<SocketAddr>()?;

    let state = AppState {
        users: UserServiceClient::new(Endpoint::from_shared(user_service_url)?.connect().await?),
        posts: PostServiceClient::new(Endpoint::from_shared(post_service_url)?.connect().await?),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", get(get_user))
        .route("/users/{id}/posts", get(list_posts_for_user))
        .route("/posts", post(create_post).delete(delete_post))
        .route("/posts/{id}", get(get_post))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(gateway_addr).await?;

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
