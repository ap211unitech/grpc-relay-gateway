use proto::posts::post_service_server::PostServiceServer;
use tonic::transport::Server;

use crate::service::PostStore;

mod error;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50052".parse()?;

    let post_store = PostStore::default();

    println!("gRPC PostService listening on {addr}");

    Server::builder()
        .add_service(PostServiceServer::new(post_store))
        .serve(addr)
        .await?;

    Ok(())
}
