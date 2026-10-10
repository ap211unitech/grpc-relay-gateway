use tonic::Status;

pub fn post_key(post_id: &str) -> String {
    format!("grpc-relay-gateway:post:{post_id}")
}

pub fn user_posts_key(user_id: &str) -> String {
    format!("grpc-relay-gateway:user-posts:{user_id}")
}

pub fn redis_unavailable(error: redis::RedisError) -> Status {
    Status::unavailable(format!("could not access Redis: {error}"))
}
