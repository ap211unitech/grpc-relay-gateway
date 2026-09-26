use futures::Stream;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use crate::{
    error::LockPoisoned,
    proto::{CreateUserRequest, Empty, GetUserRequest, User, user_service_server::UserService},
};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct UserStore {
    users: Arc<Mutex<HashMap<String, User>>>,
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

        let mut users = self.users.lock().map_err(LockPoisoned::from)?;
        let user = users.entry(id).or_insert(user).to_owned();

        Ok(tonic::Response::new(user))
    }

    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status> {
        let id = request.into_inner().id;

        let users = &self.users.lock().map_err(LockPoisoned::from)?;

        let user = users.get(&id);

        match user {
            Some(user) => Ok(tonic::Response::new(user.to_owned())),
            _ => Err(Status::not_found("user not found")),
        }
    }

    type ListUsersStream = Pin<Box<dyn Stream<Item = Result<User, Status>> + Send + 'static>>;

    async fn list_users(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListUsersStream>, Status> {
        let users: Vec<User> = self
            .users
            .lock()
            .map_err(LockPoisoned::from)?
            .values()
            .cloned()
            .collect();

        let stream = tokio_stream::iter(users.into_iter().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }
}
