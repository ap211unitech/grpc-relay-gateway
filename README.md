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

Users are stored in Redis and `ListUsers` streams the records indexed there.

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
- Posts are stored in Redis by `post_id` and indexed by `user_id` to support efficient per-user listing.

Both services persist data in Redis, so records are shared across service restarts.

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

Install Rust, Docker Compose, and `protoc` (`brew install protobuf` or `apt install protobuf-compiler`). Start Redis and then start each service from the workspace root in its own terminal:

```sh
docker compose up -d redis
cargo run -p user-service
cargo run -p post-service
cargo run -p api-gateway
```

Both gRPC services load `REDIS_URL` from the root `.env` file. The default is `redis://127.0.0.1:6379`; `REDIS_PORT` configures the host port published by Compose. Redis uses a Docker volume, so data remains available when the container is restarted. Stop the container with `docker compose down` (the volume is retained).

The services listen on:

- `user-service`: `127.0.0.1:50051`
- `post-service`: `127.0.0.1:50052`
- `api-gateway`: `127.0.0.1:8000`

`post-service` checks that a user exists when a post is created; if `user-service` is unavailable, the create request returns an error.

## Build

```sh
cargo build
```

## Protobuf source

The shared service definitions live in `proto/contracts/` and are compiled into the `proto` crate, which both services depend on. This keeps the gRPC schema as a single source of truth instead of duplicating message types across services.
