use std::time::SystemTime;

use prost::Message;
use redis::{AsyncCommands, aio::ConnectionManager};
use tonic::{
    Request, Response, Status,
    transport::{Channel, Endpoint},
};

use proto::{
    posts::{
        CreatePostRequest, DeletePostRequest, Empty, GetPostRequest, GetPostsForUserRequest, Post,
        PostList, post_service_server::PostService,
    },
    users::{GetUserRequest, user_service_client::UserServiceClient},
};

use crate::utils::{post_key, redis_unavailable, user_posts_key};

pub struct PostStore {
    connection: ConnectionManager,
    user_client: UserServiceClient<Channel>,
}

impl PostStore {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let connection = ConnectionManager::new(client).await?;
        let endpoint = Endpoint::from_static("http://127.0.0.1:50051");
        let user_client = UserServiceClient::new(endpoint.connect_lazy());

        Ok(Self {
            connection,
            user_client,
        })
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

        let mut user_client = self.user_client.clone();
        user_client
            .get_user(GetUserRequest {
                id: post.user_id.clone(),
            })
            .await
            .map_err(|status| match status.code() {
                tonic::Code::NotFound => {
                    Status::failed_precondition(format!("user {} does not exist", post.user_id))
                }
                _ => Status::unavailable("could not reach user service"),
            })?;

        let mut connection = self.connection.clone();
        redis::pipe()
            .atomic()
            .set(post_key(&post.id), post.encode_to_vec())
            .sadd(user_posts_key(&post.user_id), &post.id)
            .query_async::<()>(&mut connection)
            .await
            .map_err(redis_unavailable)?;

        Ok(Response::new(post))
    }

    async fn get_post(&self, request: Request<GetPostRequest>) -> Result<Response<Post>, Status> {
        let post_id = request.into_inner().post_id;
        let mut connection = self.connection.clone();
        let encoded: Option<Vec<u8>> = connection
            .get(post_key(&post_id))
            .await
            .map_err(redis_unavailable)?;

        match encoded {
            Some(encoded) => Post::decode(encoded.as_slice())
                .map(Response::new)
                .map_err(|error| Status::internal(format!("stored post is invalid: {error}"))),
            None => Err(Status::not_found("post not found")),
        }
    }

    async fn list_posts_for_user(
        &self,
        request: Request<GetPostsForUserRequest>,
    ) -> Result<Response<PostList>, Status> {
        let user_id = request.into_inner().user_id;
        let mut connection = self.connection.clone();
        let post_ids: Vec<String> = connection
            .smembers(user_posts_key(&user_id))
            .await
            .map_err(redis_unavailable)?;

        if post_ids.is_empty() {
            return Ok(Response::new(PostList { posts: Vec::new() }));
        }

        let keys: Vec<String> = post_ids.iter().map(|id| post_key(id)).collect();
        let encoded_posts: Vec<Option<Vec<u8>>> =
            connection.mget(keys).await.map_err(redis_unavailable)?;
        let posts = encoded_posts
            .into_iter()
            .flatten()
            .map(|encoded| {
                Post::decode(encoded.as_slice())
                    .map_err(|error| Status::internal(format!("stored post is invalid: {error}")))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Response::new(PostList { posts }))
    }

    async fn delete_post(
        &self,
        request: Request<DeletePostRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let mut connection = self.connection.clone();
        let encoded: Option<Vec<u8>> = connection
            .get(post_key(&req.post_id))
            .await
            .map_err(redis_unavailable)?;
        let Some(encoded) = encoded else {
            return Ok(Response::new(Empty {}));
        };
        let post = Post::decode(encoded.as_slice())
            .map_err(|error| Status::internal(format!("stored post is invalid: {error}")))?;

        if post.user_id != req.user_id {
            return Err(Status::permission_denied("you do not own this post"));
        }

        redis::pipe()
            .atomic()
            .del(post_key(&req.post_id))
            .srem(user_posts_key(&post.user_id), &req.post_id)
            .query_async::<()>(&mut connection)
            .await
            .map_err(redis_unavailable)?;

        Ok(Response::new(Empty {}))
    }
}
