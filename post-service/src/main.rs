use tonic::transport::Server;

use crate::{proto::post_service_server::PostServiceServer, service::PostStore};

mod error;
mod proto;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50052".parse()?;

    let user_store = PostStore::default();

    println!("gRPC PostService listening on {addr}");

    Server::builder()
        .add_service(PostServiceServer::new(user_store))
        .serve(addr)
        .await?;

    Ok(())
}
