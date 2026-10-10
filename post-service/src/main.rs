use proto::posts::post_service_server::PostServiceServer;
use std::env;
use tonic::transport::Server;

use crate::service::PostStore;

mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    let addr = env::var("POST_SERVICE_URL")?.parse()?;

    let redis_url = env::var("REDIS_URL")?;
    let user_service_url = env::var("USER_SERVICE_URL")?;
    let post_store = PostStore::new(&redis_url, &user_service_url).await?;

    println!("gRPC PostService listening on {addr}");

    Server::builder()
        .add_service(PostServiceServer::new(post_store))
        .serve(addr)
        .await?;

    Ok(())
}
