#!/usr/bin/env bash
set -euo pipefail

for tool in oha curl jq; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "$tool is required to run the load test" >&2
        exit 1
    fi
done

if (($# > 1)); then
    echo "Usage: $0 [API_BASE_URL]" >&2
    exit 2
fi

api_base_url="${1:-${API_GATEWAY_URL:-}}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
env_file="$script_dir/../.env"
if [[ -z "$api_base_url" && -f "$env_file" ]]; then
    api_base_url="$(awk -F= '$1 == "API_GATEWAY_URL" { sub(/^[^=]*=/, ""); print; exit }' "$env_file")"
fi
api_base_url="${api_base_url:-http://127.0.0.1:8000}"
api_base_url="${api_base_url#"${api_base_url%%[![:space:]]*}"}"
api_base_url="${api_base_url%"${api_base_url##*[![:space:]]}"}"
api_base_url="${api_base_url%/}"

case "$api_base_url" in
    http://*|https://*) ;;
    *://*)
        echo "API base URL must use http:// or https://" >&2
        exit 2
        ;;
    *)
        api_base_url="http://$api_base_url"
        ;;
esac

total_concurrency="${CONCURRENCY:-200}"
duration="${DURATION:-30s}"
requests="${REQUESTS:-}"
timeout="${TIMEOUT:-10s}"

if [[ ! "$total_concurrency" =~ ^[1-9][0-9]*$ ]] || ((total_concurrency < 8)); then
    echo "CONCURRENCY must be an integer of at least 8 (one per endpoint)" >&2
    exit 2
fi

user_payload='{"name":"oha load-test user","email":"oha-load-test@example.com"}'
user_id="$(
    curl --fail-with-body --silent --show-error \
        -H 'content-type: application/json' \
        -d "$user_payload" \
        "$api_base_url/users" | jq -er '.id'
)"
post_payload="{\"user_id\":\"$user_id\",\"title\":\"oha load-test post\",\"content\":\"Seed record for load testing\"}"
post_id="$(
    curl --fail-with-body --silent --show-error \
        -H 'content-type: application/json' \
        -d "$post_payload" \
        "$api_base_url/posts" | jq -er '.id'
)"
delete_post_id="$(
    curl --fail-with-body --silent --show-error \
        -H 'content-type: application/json' \
        -d "$post_payload" \
        "$api_base_url/posts" | jq -er '.id'
)"
delete_post_payload="{\"user_id\":\"$user_id\",\"post_id\":\"$delete_post_id\"}"

routes=(
    health
    create-user
    get-user
    list-users
    create-post
    get-post
    list-posts-for-user
    delete-post
)

# Shuffle the launch order while keeping one workload for every endpoint.
for ((i = ${#routes[@]} - 1; i > 0; i--)); do
    j=$((RANDOM % (i + 1)))
    route="${routes[i]}"
    routes[i]="${routes[j]}"
    routes[j]="$route"
done

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
pids=()
labels=()
logs=()
failures=0

for i in "${!routes[@]}"; do
    route="${routes[i]}"
    route_concurrency=$((total_concurrency / ${#routes[@]}))
    if ((i < total_concurrency % ${#routes[@]})); then
        route_concurrency=$((route_concurrency + 1))
    fi

    args=(--no-tui --stats-success-breakdown -c "$route_concurrency" -t "$timeout")
    if [[ -n "$requests" ]]; then
        args+=(-n "$requests")
    else
        args+=(-z "$duration")
    fi

    case "$route" in
        health)
            args+=(-m GET)
            url="$api_base_url/health"
            ;;
        create-user)
            args+=(-m POST -T application/json -d "$user_payload")
            url="$api_base_url/users"
            ;;
        get-user)
            args+=(-m GET)
            url="$api_base_url/users/$user_id"
            ;;
        list-users)
            args+=(-m GET)
            url="$api_base_url/users"
            ;;
        create-post)
            args+=(-m POST -T application/json -d "$post_payload")
            url="$api_base_url/posts"
            ;;
        get-post)
            args+=(-m GET)
            url="$api_base_url/posts/$post_id"
            ;;
        list-posts-for-user)
            args+=(-m GET)
            url="$api_base_url/users/$user_id/posts"
            ;;
        delete-post)
            args+=(-m DELETE -T application/json -d "$delete_post_payload")
            url="$api_base_url/posts"
            ;;
    esac

    log="$tmp_dir/$route.log"
    printf 'Starting %-20s with %s concurrent requests\n' "$route" "$route_concurrency"
    oha "${args[@]}" "$url" >"$log" 2>&1 &
    pids+=("$!")
    labels+=("$route")
    logs+=("$log")
done

for i in "${!pids[@]}"; do
    if wait "${pids[i]}"; then
        printf '\n=== %s ===\n' "${labels[i]}"
    else
        printf '\n=== %s (oha failed) ===\n' "${labels[i]}" >&2
        failures=1
    fi
    cat "${logs[i]}"
done

if ((failures)); then
    exit 1
fi
