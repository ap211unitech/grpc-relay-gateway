use proto::posts::post_service_server::PostServiceServer;
use std::env;
use tonic::transport::Server;

use crate::service::PostStore;

mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    let addr = "127.0.0.1:50052".parse()?;

    let redis_url = env::var("REDIS_URL")?;
    let post_store = PostStore::new(&redis_url).await?;

    println!("gRPC PostService listening on {addr}");

    Server::builder()
        .add_service(PostServiceServer::new(post_store))
        .serve(addr)
        .await?;

    Ok(())
}
