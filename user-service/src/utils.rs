use tonic::Status;

pub const USERS_KEY: &str = "grpc-relay-gateway:users";

pub fn user_key(user_id: &str) -> String {
    format!("grpc-relayuser:{user_id}")
}

pub fn redis_unavailable(error: redis::RedisError) -> Status {
    Status::unavailable(format!("could not access Redis: {error}"))
}
