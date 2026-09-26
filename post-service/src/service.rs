use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use tonic::{Request, Response, Status};

use crate::{
    error::LockPoisoned,
    proto::{CreatePostRequest, Post, post_service_server::PostService},
};

#[derive(Default)]
pub struct PostStore {
    posts: Arc<Mutex<HashMap<String, Post>>>,
    by_user: Arc<Mutex<HashMap<String, Vec<String>>>>,
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
