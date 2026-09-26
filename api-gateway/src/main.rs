use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use proto::{
    posts::{
        CreatePostRequest, DeletePostRequest, GetPostRequest, GetPostsForUserRequest,
        post_service_client::PostServiceClient,
    },
    users::user_service_client::UserServiceClient,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tonic::transport::{Channel, Endpoint};

use crate::handlers::users::{create_user, get_user, list_users};

mod errors;
mod handlers;

#[derive(Clone)]
struct AppState {
    users: UserServiceClient<Channel>,
    posts: PostServiceClient<Channel>,
}

#[derive(Debug, Deserialize)]
struct CreatePostBody {
    user_id: String,
    title: String,
    content: String,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
struct DeletePostQuery {
    user_id: String,
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
        // .route("/users/{id}/posts", get(list_posts_for_user))
        // .route("/posts", post(create_post))
        // .route("/posts/{id}", get(get_post).delete(delete_post))
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

// async fn create_post(State(state): State<AppState>, Json(body): Json<CreatePostBody>) -> Response {
//     let mut client = state.posts.clone();
//     match client
//         .create_post(CreatePostRequest {
//             user_id: body.user_id,
//             title: body.title,
//             content: body.content,
//         })
//         .await
//     {
//         Ok(response) => (
//             StatusCode::CREATED,
//             Json(PostResponse::from(response.into_inner())),
//         )
//             .into_response(),
//         Err(status) => grpc_error(status),
//     }
// }

// async fn get_post(State(state): State<AppState>, Path(id): Path<String>) -> Response {
//     let mut client = state.posts.clone();
//     match client.get_post(GetPostRequest { post_id: id }).await {
//         Ok(response) => Json(PostResponse::from(response.into_inner())).into_response(),
//         Err(status) => grpc_error(status),
//     }
// }

// async fn list_posts_for_user(
//     State(state): State<AppState>,
//     Path(user_id): Path<String>,
// ) -> Response {
//     let mut client = state.posts.clone();
//     match client
//         .list_posts_for_user(GetPostsForUserRequest { user_id })
//         .await
//     {
//         Ok(response) => Json(
//             response
//                 .into_inner()
//                 .posts
//                 .into_iter()
//                 .map(PostResponse::from)
//                 .collect::<Vec<_>>(),
//         )
//         .into_response(),
//         Err(status) => grpc_error(status),
//     }
// }

// async fn delete_post(
//     State(state): State<AppState>,
//     Path(post_id): Path<String>,
//     Query(query): Query<DeletePostQuery>,
// ) -> Response {
//     let mut client = state.posts.clone();
//     match client
//         .delete_post(DeletePostRequest {
//             user_id: query.user_id,
//             post_id,
//         })
//         .await
//     {
//         Ok(_) => StatusCode::NO_CONTENT.into_response(),
//         Err(status) => grpc_error(status),
//     }
// }
