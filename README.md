# gRPC Relay Gateway

A Rust microservice showing gRPC service-to-service communication behind a REST gateway. The project includes:

- `user-service` for user management and streaming list responses
- `post-service` for post management and cross-service validation against the user service
- `api-gateway` as an Axum HTTP layer that exposes a simpler REST API
- shared protobuf contracts compiled by the `proto` crate

## Architecture

```text
Browser / curl ──HTTP/REST──▶ api-gateway :8000 ──┐
                                                    │ gRPC         │ gRPC
                                                    ▼              ▼
                                            user-service :50051   post-service :50052
                                                                      │
                                                                      └── calls user-service
                                                                          in few scenarios
```

## gRPC services

### User service — `127.0.0.1:50051`

Supported RPCs:

- `CreateUser`
- `GetUser`
- `ListUsers` (server-streaming)

The service keeps users in memory with `Arc<Mutex<HashMap<String, User>>>` and returns a stream of user records for `ListUsers`.

### Post service — `127.0.0.1:50052`

Supported RPCs:

- `CreatePost`
- `GetPost`
- `ListPostsForUser`
- `DeletePost`

Behavior details:

- `CreatePost` validates that the target `user_id` exists by calling `UserService.GetUser`.
  - missing user -> `FAILED_PRECONDITION`
  - connectivity or downstream failure -> `UNAVAILABLE`
- `DeletePost` requires the caller to send the matching `user_id` and rejects mismatches with `PERMISSION_DENIED`.
- Deleting a missing post is treated as success, which matches a forgiving REST-style DELETE flow.
- Posts are indexed both by `post_id` and by `user_id` to support efficient per-user listing.

Both services keep data in process memory.

## API gateway

The gateway runs on `127.0.0.1:8000` and translates REST requests into gRPC calls internally.

### Routes

| Method | Path               | Behavior                                   |
| ------ | ------------------ | ------------------------------------------ |
| GET    | `/health`          | Health check                               |
| POST   | `/users`           | `UserService.CreateUser`                   |
| GET    | `/users/:id`       | `UserService.GetUser`                      |
| GET    | `/users`           | `UserService.ListUsers`                    |
| POST   | `/posts`           | `PostService.CreatePost`                   |
| GET    | `/posts/:id`       | `PostService.GetPost`                      |
| GET    | `/users/:id/posts` | `PostService.ListPostsForUser`             |
| DELETE | `/posts`           | `PostService.DeletePost` using a JSON body |

### Status (gRPC-to-HTTP mapping handled in the gateway)

| gRPC status           | HTTP status |
| --------------------- | ----------- |
| `NOT_FOUND`           | 404         |
| `FAILED_PRECONDITION` | 422         |
| `PERMISSION_DENIED`   | 403         |
| `INVALID_ARGUMENT`    | 400         |
| `UNAVAILABLE`         | 503         |
| `INTERNAL`            | 500         |

## Run locally

Install Rust and `protoc` (`brew install protobuf` or `apt install protobuf-compiler`), then start each service from the workspace root in its own terminal:

```sh
cargo run -p user-service
cargo run -p post-service
cargo run -p api-gateway
```

The services listen on:

- `user-service`: `127.0.0.1:50051`
- `post-service`: `127.0.0.1:50052`
- `api-gateway`: `127.0.0.1:8000`

`post-service` connects to `user-service` during startup and exits early if it is not reachable, which makes the dependency problem visible immediately.

## Build

```sh
cargo build
```

## Protobuf source

The shared service definitions live in `proto/contracts/` and are compiled into the `proto` crate, which both services depend on. This keeps the gRPC schema as a single source of truth instead of duplicating message types across services.
