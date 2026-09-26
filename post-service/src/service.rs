use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use tonic::{
    Request, Response, Status,
    transport::{Channel, Endpoint},
};

use crate::error::LockPoisoned;
use proto::{
    posts::{CreatePostRequest, Post, post_service_server::PostService},
    users::{GetUserRequest, user_service_client::UserServiceClient},
};

pub struct PostStore {
    posts: Arc<Mutex<HashMap<String, Post>>>,
    by_user: Arc<Mutex<HashMap<String, Vec<String>>>>,
    user_client: UserServiceClient<Channel>,
}

impl Default for PostStore {
    fn default() -> Self {
        let endpoint = Endpoint::from_static("http://127.0.0.1:50051");
        let user_client = UserServiceClient::new(endpoint.connect_lazy());

        Self {
            posts: Arc::new(Mutex::new(HashMap::new())),
            by_user: Arc::new(Mutex::new(HashMap::new())),
            user_client,
        }
    }
}

#[tonic::async_trait]
impl PostService for PostStore {
    async fn create_post(
        &self,
        request: Request<CreatePostRequest>,
    ) -> Result<Response<Post>, Status> {
        let req = request.into_inner();
        let post_id = uuid::Uuid::new_v4().to_string();
        let created_at = prost_types::Timestamp::from(SystemTime::now());

        let post = Post {
            id: post_id,
            user_id: req.user_id,
            title: req.title.trim().to_string(),
            content: req.content.trim().to_string(),
            created_at: Some(created_at),
        };

        if post.title.is_empty() {
            return Err(Status::invalid_argument("title must not be empty"));
        }

        if post.content.is_empty() {
            return Err(Status::invalid_argument("content must not be empty"));
        }

        // Check if user exists
        let mut user_client = self.user_client.clone();
        user_client
            .get_user(GetUserRequest {
                id: post.user_id.clone(),
            })
            .await
            .map_err(|status| match status.code() {
                tonic::Code::NotFound => Status::failed_precondition(format!(
                    "user {} does not exist",
                    post.user_id.clone()
                )),
                _ => Status::unavailable("could not reach user service"),
            })?;

        self.posts
            .lock()
            .map_err(LockPoisoned::from)?
            .insert(post.id.clone(), post.clone());

        self.by_user
            .lock()
            .map_err(LockPoisoned::from)?
            .entry(post.user_id.clone())
            .or_default()
            .push(post.id.clone());

        Ok(Response::new(post))
    }
}
