use tonic::transport::Server;

use crate::{proto::user_service_server::UserServiceServer, service::UserStore};

mod error;
mod proto;
mod service;

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
