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

Install Rust, Docker Compose, and `protoc` (`brew install protobuf` or `apt install protobuf-compiler`). Copy the committed `.env.example` template to `.env`; services and Docker Compose read their configuration from `.env`:

```sh
cp .env.example .env
docker compose up -d redis
cargo run -p user-service
cargo run -p post-service
cargo run -p api-gateway
```

All services load their URLs from `.env`: `USER_SERVICE_URL`, `POST_SERVICE_URL`, and `API_GATEWAY_URL` configure the gRPC endpoints and gateway listener. Both gRPC services use `REDIS_URL` to connect to Redis; `REDIS_PORT` configures the host port published by Compose. Redis uses a Docker volume, so data remains available when the container is restarted. Stop the container with `docker compose down` (the volume is retained).

The services listen on:

- `user-service`: `127.0.0.1:50051`
- `post-service`: `127.0.0.1:50052`
- `api-gateway`: `127.0.0.1:8000`

`post-service` checks that a user exists when a post is created; if `user-service` is unavailable, the create request returns an error.

## Load testing

Install [`oha`](https://github.com/hatoo/oha) and `jq` (for example, `brew install oha jq`), start the services, then run the benchmark script:

```sh
./scripts/load-test.sh
```

The script seeds a user and post, then concurrently benchmarks every API endpoint (health, user create/get/list, and post create/get/list/delete). It randomizes the order in which the per-endpoint workloads start. By default it runs each endpoint for 30 seconds with 40 total concurrent connections split across the endpoints. It reads `API_GATEWAY_URL` from the environment or `.env`; optionally pass the API base URL as the first argument:

```sh
./scripts/load-test.sh http://127.0.0.1:8000
```

You can configure the run with `CONCURRENCY` (at least 8, default `200`), `DURATION` (default `30s`), `REQUESTS` (optional fixed request count per endpoint instead of duration), and `TIMEOUT` (default `10s`). For example:

```sh
CONCURRENCY=800 DURATION=60s ./scripts/load-test.sh
```

## Load Test Results

| Endpoint            | Concurrency | P50        | P95        | P99        | Status Code | Error Rate |
| ------------------- | ----------- | ---------- | ---------- | ---------- | ----------- | ---------- |
| create-user         | 25          | 873.7 ms   | 1,374.1 ms | 1,707.8 ms | 201         | 0%         |
| delete-post         | 25          | 41.19 ms   | 88.66 ms   | 118.63 ms  | 204         | 0%         |
| list-posts-for-user | 25          | 62.77 ms   | 144.24 ms  | 187.66 ms  | 200         | 0%         |
| create-post         | 25          | 767.6 ms   | 1,193.1 ms | 1,500.9 ms | 201         | 0%         |
| list-users          | 25          | 2,021.0 ms | 2,709.4 ms | 3,064.3 ms | 200         | 0%         |
| get-post            | 25          | 41.38 ms   | 88.88 ms   | 118.82 ms  | 200         | 0%         |
| get-user            | 25          | 875.8 ms   | 1,394.9 ms | 1,707.8 ms | 200         | 0%         |
| health              | 25          | 0.694 ms   | 5.20 ms    | 13.84 ms   | 200         | 0%         |

## Build

```sh
cargo build
```

## Protobuf source

The shared service definitions live in `proto/contracts/` and are compiled into the `proto` crate, which both services depend on. This keeps the gRPC schema as a single source of truth instead of duplicating message types across services.
