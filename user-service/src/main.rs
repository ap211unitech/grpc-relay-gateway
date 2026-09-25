tonic::include_proto!("users");

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tonic::{Request, Response, Status, transport::Server};
use user_service_server::{UserService, UserServiceServer};

#[derive(Debug, Default)]
struct UserStore {
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

        let user = User {
            id: id.clone(),
            name: user.name,
            email: user.email,
        };

        let mut users = self
            .users
            .lock()
            .map_err(|_| Status::internal("user store lock poisoned"))?;
        let user = users.entry(id).or_insert(user).to_owned();

        Ok(tonic::Response::new(user))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    let user_store = UserStore::default();

    println!("gRPC UserService listening on {addr}");

    Server::builder()
        .add_service(UserServiceServer::new(user_store))
        .serve(addr)
        .await?;

    Ok(())
}
