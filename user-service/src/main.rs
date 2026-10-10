use proto::users::user_service_server::UserServiceServer;
use std::env;
use tonic::transport::Server;

use crate::service::UserStore;

mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    let addr = "127.0.0.1:50051".parse()?;

    let redis_url = env::var("REDIS_URL")?;
    let user_store = UserStore::new(&redis_url).await?;

    println!("gRPC UserService listening on {addr}");

    Server::builder()
        .add_service(UserServiceServer::new(user_store))
        .serve(addr)
        .await?;

    Ok(())
}
