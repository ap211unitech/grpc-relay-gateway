# gRPC Relay Gateway

A multi-service backend in Rust, built to demonstrate real service-to-service gRPC communication. Two backend services are implemented and talk to each other over gRPC; a REST gateway in front of them
is designed for client's simplicity.

## Architecture

```
Browser/curl ──HTTP/REST──▶ api-gateway ──┐
                                            │ gRPC        │ gRPC
                                            ▼             ▼
                                    UserService    PostService
                                    :50051          :50052
                                                       │
                                                       └──gRPC call to UserService
                                                          in few scenarios

```

## gRPC Services

### UserService — `127.0.0.1:50051`

- `CreateUser`, `GetUser`
- `ListUsers` — server-streaming, returned as an ongoing stream of `User`
  messages rather than one bulk response.

### PostService — `127.0.0.1:50052`

- `CreatePost`, `GetPost`, `ListPostsForUser`, `UpdatePost`, `DeletePost`
- **Cross-service validation**: before creating a post, `PostService` calls
  `UserService.GetUser` to confirm the author exists. A missing user maps to
  `FAILED_PRECONDITION` (the request was well-formed but its dependency
  doesn't hold); a genuine network/connection failure maps to `UNAVAILABLE`
  instead — the two are deliberately not conflated, since they call for
  different client behavior (fix the request vs. retry later).
- **Ownership checks**: `UpdatePost` and `DeletePost` require the caller to
  supply a `user_id` and reject the request with `PERMISSION_DENIED` if it
  doesn't match the post's author. This is authorization _by convention_,
  not by authentication — there's no verified identity behind `user_id` yet,
  which is exactly the kind of thing a real auth layer would need to close.
- **Idempotent delete**: deleting an already-deleted or nonexistent post
  returns success rather than `NOT_FOUND`, matching common REST/HTTP
  `DELETE` semantics.
- **Secondary index**: posts are stored twice in memory — once by `post_id`
  for direct lookups, and once by `user_id` for `ListPostsForUser` — trading
  a small consistency burden (every write touches both maps) for O(1)
  lookups instead of a full scan.

Both services store data in memory (`Arc<Mutex<HashMap<...>>>`).

## API gateway

An `axum`-based HTTP service that translates REST calls into the gRPC calls
internally, so the system is usable from a browser or `curl` without a gRPC
client. Routes exposed:

| Method | Path               | Calls                          |
| ------ | ------------------ | ------------------------------ |
| POST   | `/users`           | `UserService.CreateUser`       |
| GET    | `/users/:id`       | `UserService.GetUser`          |
| GET    | `/users`           | `UserService.ListUsers`        |
| POST   | `/posts`           | `PostService.CreatePost`       |
| GET    | `/posts/:id`       | `PostService.GetPost`          |
| GET    | `/users/:id/posts` | `PostService.ListPostsForUser` |
| DELETE | `/posts/:id`       | `PostService.DeletePost`       |

gRPC-status-to-HTTP-status mapping:

| gRPC status           | HTTP status |
| --------------------- | ----------- |
| `NOT_FOUND`           | 404         |
| `FAILED_PRECONDITION` | 422         |
| `PERMISSION_DENIED`   | 403         |
| `INVALID_ARGUMENT`    | 400         |
| `UNAVAILABLE`         | 503         |
| `INTERNAL`            | 500         |

Planned: request timeouts on the gateway's gRPC clients so a downstream
outage returns a clean 503 instead of hanging, and a request ID threaded
through gateway → post-service → user-service for tracing a single request
across process logs.

## Run locally

Install Rust and `protoc` (`brew install protobuf` / `apt install protobuf-compiler`),
then from the workspace root, in separate terminals:

```sh
cargo run -p user-service    # listens on 127.0.0.1:50051
cargo run -p post-service    # listens on 127.0.0.1:50052, requires user-service running first
```

`post-service` connects to `user-service` at startup and will fail fast if
it isn't reachable — a deliberate choice to surface the dependency
immediately rather than retrying silently.

Build everything:

```sh
cargo build
```

Service contracts live in `proto/contracts/` and are compiled into the
shared `proto` crate, which both services depend on — this keeps the gRPC
schema as a single source of truth rather than duplicating message types
per service.
