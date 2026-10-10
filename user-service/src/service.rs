use futures::Stream;
use prost::Message;
use proto::users::{
    CreateUserRequest, Empty, GetUserRequest, User, user_service_server::UserService,
};
use redis::{AsyncCommands, aio::ConnectionManager};
use std::{pin::Pin, time::SystemTime};
use tonic::{Request, Response, Status};

use crate::utils::{USERS_KEY, redis_unavailable, user_key};

pub struct UserStore {
    connection: ConnectionManager,
}

impl UserStore {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let connection = ConnectionManager::new(client).await?;
        Ok(Self { connection })
    }
}

#[tonic::async_trait]
impl UserService for UserStore {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<User>, Status> {
        let user = request.into_inner();

        if user.name.trim().is_empty() {
            return Err(Status::invalid_argument("name must not be empty"));
        }
        if user.email.trim().is_empty() {
            return Err(Status::invalid_argument("email must not be empty"));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let created_at = prost_types::Timestamp::from(SystemTime::now());

        let user = User {
            id: id.clone(),
            name: user.name,
            email: user.email,
            created_at: Some(created_at),
        };

        let mut connection = self.connection.clone();
        redis::pipe()
            .atomic()
            .set(user_key(&id), user.encode_to_vec())
            .sadd(USERS_KEY, id)
            .query_async::<()>(&mut connection)
            .await
            .map_err(redis_unavailable)?;

        Ok(Response::new(user))
    }

    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status> {
        let id = request.into_inner().id;
        let mut connection = self.connection.clone();
        let encoded: Option<Vec<u8>> = connection
            .get(user_key(&id))
            .await
            .map_err(redis_unavailable)?;

        match encoded {
            Some(encoded) => User::decode(encoded.as_slice())
                .map(Response::new)
                .map_err(|error| Status::internal(format!("stored user is invalid: {error}"))),
            None => Err(Status::not_found("user not found")),
        }
    }

    type ListUsersStream = Pin<Box<dyn Stream<Item = Result<User, Status>> + Send + 'static>>;

    async fn list_users(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListUsersStream>, Status> {
        let mut connection = self.connection.clone();
        let user_ids: Vec<String> = connection
            .smembers(USERS_KEY)
            .await
            .map_err(redis_unavailable)?;

        let users = if user_ids.is_empty() {
            Vec::new()
        } else {
            let keys: Vec<String> = user_ids.iter().map(|id| user_key(id)).collect();
            let encoded_users: Vec<Option<Vec<u8>>> =
                connection.mget(keys).await.map_err(redis_unavailable)?;
            encoded_users
                .into_iter()
                .flatten()
                .map(|encoded| {
                    User::decode(encoded.as_slice()).map_err(|error| {
                        Status::internal(format!("stored user is invalid: {error}"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let stream = tokio_stream::iter(users.into_iter().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }
}
