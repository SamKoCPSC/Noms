#!/usr/bin/env bash
set -euo pipefail

readonly REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly COMPOSE_FILE="$REPOSITORY_ROOT/docker-compose.yml"
readonly RUNTIME_ROOT="${NOMS_RUNTIME_ROOT:-${TMPDIR:-/tmp}/noms-dev}"
# Locks are deliberately independent of the configurable runtime root. Two
# callers from the same host user must contend for the same worktree/startup
# locks even if they store disposable metadata in different locations.
readonly LOCK_ROOT="/tmp/noms-dev-locks-${UID}"

NOMS_UP_PROJECT_CREATED=false
NOMS_UP_NORMAL_SHUTDOWN=false
NOMS_UP_INSTANCE_ID=""
NOMS_UP_RUNTIME_DIR=""
NOMS_UP_WORKTREE=""
NOMS_UP_WORKTREE_LOCK=""
NOMS_UP_INSTANCE_TOKEN=""
NOMS_UP_METADATA_FILE=""
NOMS_UP_COMPOSE_LOG=""
NOMS_UP_CREATED_AT=""
NOMS_UP_SESSION_COOKIE_NAME=""
NOMS_UP_APPLICATION_PORT=""
NOMS_UP_POSTGRES_PORT=""
NOMS_UP_MINIO_PORT=""
NOMS_UP_MINIO_CONSOLE_PORT=""
NOMS_UP_MOCK_OAUTH_PORT=""
NOMS_UP_STARTUP_SLOT=""

usage() {
    cat <<'EOF'
Usage:
  scripts/dev-environment.sh up [instance]
  scripts/dev-environment.sh down <instance>
  scripts/dev-environment.sh status <instance>
  scripts/dev-environment.sh logs <instance>
  scripts/dev-environment.sh instances
  scripts/dev-environment.sh normalize-instance <instance>
EOF
}

error() {
    echo "ERROR: $*" >&2
}

normalize_instance() {
    local requested_name="${1-}"
    if [[ "$requested_name" == instance=* ]]; then
        requested_name="${requested_name#instance=}"
    fi
    if [[ -z "$requested_name" ]]; then
        error "Instance names cannot be empty."
        return 1
    fi

    local instance_id="$requested_name"
    if [[ "$instance_id" != noms-* ]]; then
        instance_id="noms-$instance_id"
    fi

    if [[ ! "$instance_id" =~ ^noms-[a-z0-9][a-z0-9_-]{0,49}$ ]]; then
        error "Invalid instance '$requested_name'. Use 1-50 lowercase letters, digits, dashes, or underscores; the first character must be alphanumeric."
        return 1
    fi

    printf '%s\n' "$instance_id"
}

random_hex() {
    local byte_count="${1:-8}"
    od -An -N "$byte_count" -tx1 /dev/urandom | tr -d '[:space:]'
}

canonical_worktree() {
    local git_root
    git_root="$(git -C "$REPOSITORY_ROOT" rev-parse --show-toplevel)"
    (cd "$git_root" && pwd -P)
}

worktree_hash() {
    local worktree="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        printf '%s' "$worktree" | sha256sum | awk '{print $1}'
    else
        printf '%s' "$worktree" | shasum -a 256 | awk '{print $1}'
    fi
}

metadata_value() {
    local metadata_file="$1"
    local key="$2"
    sed -n "s/^${key}=//p" "$metadata_file" | head -n 1
}

require_command() {
    local command_name="$1"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        error "Required command '$command_name' was not found."
        return 1
    fi
}

ensure_docker() {
    require_command docker
    if ! docker info >/dev/null 2>&1; then
        error "Docker is not running. Start Docker Desktop, Rancher Desktop, or your Docker engine and retry."
        return 1
    fi
    if ! docker compose version >/dev/null 2>&1; then
        error "Docker Compose v2 is required ('docker compose')."
        return 1
    fi
}

project_container_ids() {
    local instance_id="$1"
    docker ps --all --quiet \
        --filter "label=com.docker.compose.project=$instance_id"
}

validate_project_ownership() {
    local instance_id="$1"
    local expected_token="${2-}"
    local container_id
    local found=false

    while IFS= read -r container_id; do
        [[ -n "$container_id" ]] || continue
        found=true
        local is_noms
        local labeled_instance
        local labeled_token
        is_noms="$(docker inspect --format '{{index .Config.Labels "com.noms.dev-environment"}}' "$container_id")"
        labeled_instance="$(docker inspect --format '{{index .Config.Labels "com.noms.dev-instance"}}' "$container_id")"
        labeled_token="$(docker inspect --format '{{index .Config.Labels "com.noms.dev-token"}}' "$container_id")"
        if [[ "$is_noms" != "true" || "$labeled_instance" != "$instance_id" ]]; then
            error "Refusing to target Compose project '$instance_id': container '$container_id' is not owned by that Noms development instance."
            return 2
        fi
        if [[ -n "$expected_token" && "$expected_token" != "recovery" &&
            "$labeled_token" != "$expected_token" ]]; then
            error "Refusing to target Compose project '$instance_id': its ownership token does not match the private instance metadata."
            return 2
        fi
    done < <(project_container_ids "$instance_id")

    "$found"
}

compose_command() {
    local instance_id="$1"
    local runtime_dir="$2"
    local worktree="$3"
    local instance_token="$4"
    shift 4

    NOMS_INSTANCE_ID="$instance_id" \
    NOMS_PROJECT_NAME="$instance_id" \
    NOMS_RUNTIME_DIR="$runtime_dir" \
    NOMS_WORKTREE="$worktree" \
    NOMS_INSTANCE_TOKEN="$instance_token" \
    NOMS_APPLICATION_PORT="${NOMS_UP_APPLICATION_PORT:-0}" \
    NOMS_MINIO_PORT="${NOMS_UP_MINIO_PORT:-0}" \
    NOMS_MINIO_CONSOLE_PORT="${NOMS_UP_MINIO_CONSOLE_PORT:-0}" \
    NOMS_MOCK_OAUTH_PORT="${NOMS_UP_MOCK_OAUTH_PORT:-0}" \
        docker compose \
            --project-name "$instance_id" \
            --file "$worktree/docker-compose.yml" \
            "$@"
}

release_instance_locks() {
    local instance_id="$1"
    local locks_root="$LOCK_ROOT/worktrees"
    local lock_directory

    [[ -d "$locks_root" ]] || return 0
    while IFS= read -r lock_directory; do
        local lock_name="${lock_directory##*/}"
        [[ "$lock_name" =~ ^[0-9a-f]{64}$ ]] || continue
        local owner_file="$lock_directory/owner"
        if [[ -f "$owner_file" ]] &&
            [[ "$(metadata_value "$owner_file" INSTANCE_ID)" == "$instance_id" ]]; then
            rm -rf -- "$lock_directory"
        fi
    done < <(find "$locks_root" -mindepth 1 -maxdepth 1 -type d -print)
}

down_instance() {
    local requested_name="${1-}"
    local instance_id
    instance_id="$(normalize_instance "$requested_name")"
    ensure_docker

    local runtime_dir="$RUNTIME_ROOT/$instance_id"
    local metadata_file="$runtime_dir/metadata"
    local worktree="$REPOSITORY_ROOT"
    local instance_token="recovery"

    if [[ -f "$metadata_file" ]]; then
        local recorded_instance
        local recorded_project
        recorded_instance="$(metadata_value "$metadata_file" INSTANCE_ID)"
        recorded_project="$(metadata_value "$metadata_file" PROJECT_NAME)"
        if [[ "$recorded_instance" != "$instance_id" || "$recorded_project" != "$instance_id" ]]; then
            error "Refusing cleanup: metadata for '$instance_id' does not match its exact project identity."
            return 1
        fi
        instance_token="$(metadata_value "$metadata_file" INSTANCE_TOKEN)"
        [[ -n "$instance_token" ]] || instance_token="recovery"
    fi

    local has_containers=false
    if validate_project_ownership "$instance_id" "$instance_token"; then
        has_containers=true
    else
        local ownership_status=$?
        if [[ "$ownership_status" -ne 1 ]]; then
            return "$ownership_status"
        fi
    fi

    if "$has_containers"; then
        echo "Stopping isolated Noms instance: $instance_id"
        compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" \
            down --volumes --remove-orphans
    elif [[ ! -d "$runtime_dir" ]]; then
        echo "No Noms development instance named '$instance_id' was found."
        return 0
    fi

    rm -rf -- "$runtime_dir"
    release_instance_locks "$instance_id"
    echo "Removed isolated Noms instance: $instance_id"
}

port_for_service() {
    local instance_id="$1"
    local runtime_dir="$2"
    local worktree="$3"
    local instance_token="$4"
    local service="$5"
    local container_port="$6"
    local binding
    local host_port

    binding="$(compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" \
        port "$service" "$container_port")"
    host_port="${binding##*:}"
    if [[ ! "$host_port" =~ ^[0-9]+$ ]] || ((host_port < 1 || host_port > 65535)); then
        error "Could not discover the host port for $service:$container_port (received '$binding')."
        return 1
    fi
    printf '%s\n' "$host_port"
}

container_for_service() {
    local instance_id="$1"
    local runtime_dir="$2"
    local worktree="$3"
    local instance_token="$4"
    local service="$5"
    local container_id

    container_id="$(compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" \
        ps --all --quiet "$service")"
    if [[ ! "$container_id" =~ ^[0-9a-f]{12,64}$ ]]; then
        error "Could not identify the created container for service '$service'."
        return 1
    fi
    printf '%s\n' "$container_id"
}

start_created_service() {
    local container_id
    container_id="$(container_for_service "$@")"
    docker start "$container_id" >/dev/null
}

stop_created_service() {
    local container_id
    container_id="$(container_for_service "$@")"
    docker stop "$container_id" >/dev/null
}

acquire_startup_slot() {
    local instance_id="$1"
    local instance_token="$2"
    local startup_slot="$LOCK_ROOT/startup"
    local attempts="${NOMS_STARTUP_SLOT_TIMEOUT:-1800}"
    local announced_wait=false

    while ((attempts > 0)); do
        if mkdir "$startup_slot" 2>/dev/null; then
            {
                printf 'INSTANCE_ID=%s\n' "$instance_id"
                printf 'INSTANCE_TOKEN=%s\n' "$instance_token"
                printf 'PID=%s\n' "$$"
            } >"$startup_slot/owner"
            chmod 700 "$startup_slot"
            chmod 600 "$startup_slot/owner"
            NOMS_UP_STARTUP_SLOT="$startup_slot"
            return 0
        fi

        local owner_pid=""
        local owner_instance="another instance"
        if [[ -f "$startup_slot/owner" ]]; then
            owner_pid="$(metadata_value "$startup_slot/owner" PID)"
            owner_instance="$(metadata_value "$startup_slot/owner" INSTANCE_ID)"
        fi
        if [[ ! "$owner_pid" =~ ^[0-9]+$ ]] || ! kill -0 "$owner_pid" 2>/dev/null; then
            rm -rf -- "$startup_slot"
            continue
        fi

        if [[ "$announced_wait" == false ]]; then
            echo "Waiting for initial build capacity held by '$owner_instance'..." >&2
            announced_wait=true
        fi
        sleep 1
        attempts=$((attempts - 1))
    done

    error "Timed out waiting for initial build capacity."
    return 1
}

release_startup_slot() {
    local startup_slot="${NOMS_UP_STARTUP_SLOT-}"
    [[ -n "$startup_slot" && -f "$startup_slot/owner" ]] || return 0
    if [[ "$(metadata_value "$startup_slot/owner" INSTANCE_ID)" == "$NOMS_UP_INSTANCE_ID" ]] &&
        [[ "$(metadata_value "$startup_slot/owner" INSTANCE_TOKEN)" == "$NOMS_UP_INSTANCE_TOKEN" ]]; then
        rm -rf -- "$startup_slot"
    fi
    NOMS_UP_STARTUP_SLOT=""
}

write_runtime_environment() {
    local runtime_environment_file="$1"
    local instance_id="$2"
    local session_cookie_name="$3"
    local session_secret="$4"
    local application_port="$5"
    local minio_port="$6"
    local mock_oauth_port="$7"
    local port_bootstrap="${8:-0}"

    local application_url="http://127.0.0.1:$application_port"
    local minio_public_url="http://127.0.0.1:$minio_port/noms-media"
    local mock_oauth_url="http://127.0.0.1:$mock_oauth_port"

    {
        printf 'NOMS_ENV=local\n'
        printf 'NOMS_INSTANCE_ID=%s\n' "$instance_id"
        printf 'NOMS_PORT_BOOTSTRAP=%s\n' "$port_bootstrap"
        printf 'NOMS_SESSION_COOKIE_NAME=%s\n' "$session_cookie_name"
        printf 'BASE_URL=%s\n' "$application_url"
        printf 'DATABASE_URL=postgres://noms:noms_local_pass@postgres:5432/noms\n'
        printf 'PGHOST=postgres\n'
        printf 'PGPORT=5432\n'
        printf 'PGDATABASE=noms\n'
        printf 'PGUSER=noms\n'
        printf 'PGPASSWORD=noms_local_pass\n'
        printf 'R2_ENDPOINT=http://minio:9000\n'
        printf 'R2_ACCESS_KEY_ID=noms-local-key\n'
        printf 'R2_SECRET_ACCESS_KEY=noms-local-secret\n'
        printf 'R2_BUCKET=noms-media\n'
        printf 'R2_PUBLIC_URL=%s\n' "$minio_public_url"
        printf 'GOOGLE_CLIENT_ID=google\n'
        printf 'GOOGLE_CLIENT_SECRET=secret\n'
        printf 'GOOGLE_AUTH_URL=%s/google/authorize\n' "$mock_oauth_url"
        printf 'GOOGLE_TOKEN_URL=http://mock-oauth:8080/google/token\n'
        printf 'GOOGLE_USERINFO_URL=http://mock-oauth:8080/google/userinfo\n'
        printf 'GITHUB_CLIENT_ID=github\n'
        printf 'GITHUB_CLIENT_SECRET=secret\n'
        printf 'GITHUB_OAUTH_EXTRA_SCOPES=openid\n'
        printf 'GITHUB_AUTH_URL=%s/github/authorize\n' "$mock_oauth_url"
        printf 'GITHUB_TOKEN_URL=http://mock-oauth:8080/github/token\n'
        printf 'GITHUB_USERINFO_URL=http://mock-github-api:8080/github/userinfo\n'
        printf 'GITHUB_EMAILS_URL=http://mock-github-api:8080/github/emails\n'
        printf 'APPLE_SERVICES_ID=\n'
        printf 'APPLE_TEAM_ID=\n'
        printf 'APPLE_KEY_ID=\n'
        printf 'APPLE_PRIVATE_KEY=\n'
        printf 'SESSION_SECRET=%s\n' "$session_secret"
        printf 'MINIO_CORS_ORIGIN=%s\n' "$application_url"
    } >"$runtime_environment_file"
    chmod 600 "$runtime_environment_file"
}

write_metadata() {
    local metadata_file="$1"
    local state="$2"
    local instance_id="$3"
    local instance_token="$4"
    local worktree="$5"
    local process_id="$6"
    local created_at="$7"
    local application_port="${8-}"
    local postgres_port="${9-}"
    local minio_port="${10-}"
    local minio_console_port="${11-}"
    local mock_oauth_port="${12-}"
    local session_cookie_name="${13-}"
    local application_url=""
    local mock_oauth_url=""

    if [[ -n "$application_port" ]]; then
        application_url="http://127.0.0.1:$application_port"
    fi
    if [[ -n "$mock_oauth_port" ]]; then
        mock_oauth_url="http://127.0.0.1:$mock_oauth_port"
    fi

    {
        printf 'INSTANCE_ID=%s\n' "$instance_id"
        printf 'PROJECT_NAME=%s\n' "$instance_id"
        printf 'INSTANCE_TOKEN=%s\n' "$instance_token"
        printf 'STATE=%s\n' "$state"
        printf 'WORKTREE=%s\n' "$worktree"
        printf 'PID=%s\n' "$process_id"
        printf 'CREATED_AT=%s\n' "$created_at"
        printf 'RUNTIME_DIR=%s\n' "${metadata_file%/metadata}"
        printf 'APP_URL=%s\n' "$application_url"
        printf 'APP_PORT=%s\n' "$application_port"
        printf 'POSTGRES_PORT=%s\n' "$postgres_port"
        printf 'MINIO_PORT=%s\n' "$minio_port"
        printf 'MINIO_CONSOLE_PORT=%s\n' "$minio_console_port"
        printf 'MOCK_OAUTH_URL=%s\n' "$mock_oauth_url"
        printf 'MOCK_OAUTH_PORT=%s\n' "$mock_oauth_port"
        printf 'SESSION_COOKIE_NAME=%s\n' "$session_cookie_name"
    } >"$metadata_file"
    chmod 600 "$metadata_file"
}

capture_up_failure_diagnostics() {
    {
        echo
        echo "===== failure diagnostics for $NOMS_UP_INSTANCE_ID ====="
        compose_command "$NOMS_UP_INSTANCE_ID" "$NOMS_UP_RUNTIME_DIR" \
            "$NOMS_UP_WORKTREE" "$NOMS_UP_INSTANCE_TOKEN" ps --all
        local container_id
        while IFS= read -r container_id; do
            [[ -n "$container_id" ]] || continue
            docker inspect --format \
                '{{.Name}} exit={{.State.ExitCode}} oom_killed={{.State.OOMKilled}} error={{json .State.Error}}' \
                "$container_id"
        done < <(project_container_ids "$NOMS_UP_INSTANCE_ID")
        compose_command "$NOMS_UP_INSTANCE_ID" "$NOMS_UP_RUNTIME_DIR" \
            "$NOMS_UP_WORKTREE" "$NOMS_UP_INSTANCE_TOKEN" logs --no-color
    } >>"$NOMS_UP_COMPOSE_LOG" 2>&1 || true
}

cleanup_up_instance() {
    local exit_status="$1"
    trap - EXIT INT TERM
    set +e

    local runtime_still_exists=false
    [[ -d "$NOMS_UP_RUNTIME_DIR" ]] && runtime_still_exists=true

    if [[ "$NOMS_UP_PROJECT_CREATED" == true ]]; then
        if [[ "$exit_status" -ne 0 && "$NOMS_UP_NORMAL_SHUTDOWN" == false &&
            "$runtime_still_exists" == true ]]; then
            capture_up_failure_diagnostics
        fi
        compose_command "$NOMS_UP_INSTANCE_ID" "$NOMS_UP_RUNTIME_DIR" \
            "$NOMS_UP_WORKTREE" "$NOMS_UP_INSTANCE_TOKEN" \
            down --volumes --remove-orphans >>"$NOMS_UP_COMPOSE_LOG" 2>&1 || true
    fi

    release_startup_slot
    rm -rf -- "$NOMS_UP_WORKTREE_LOCK"
    runtime_still_exists=false
    [[ -d "$NOMS_UP_RUNTIME_DIR" ]] && runtime_still_exists=true
    if [[ "$NOMS_UP_NORMAL_SHUTDOWN" == true ]]; then
        rm -rf -- "$NOMS_UP_RUNTIME_DIR"
    elif [[ "$runtime_still_exists" == true ]]; then
        write_metadata "$NOMS_UP_METADATA_FILE" failed "$NOMS_UP_INSTANCE_ID" \
            "$NOMS_UP_INSTANCE_TOKEN" "$NOMS_UP_WORKTREE" "$$" "$NOMS_UP_CREATED_AT" \
            "$NOMS_UP_APPLICATION_PORT" "$NOMS_UP_POSTGRES_PORT" "$NOMS_UP_MINIO_PORT" \
            "$NOMS_UP_MINIO_CONSOLE_PORT" "$NOMS_UP_MOCK_OAUTH_PORT" \
            "$NOMS_UP_SESSION_COOKIE_NAME"
        error "Instance '$NOMS_UP_INSTANCE_ID' failed. Diagnostics were preserved at $NOMS_UP_COMPOSE_LOG"
    fi

    exit "$exit_status"
}

handle_up_signal() {
    NOMS_UP_NORMAL_SHUTDOWN=true
    exit 130
}

up_instance() {
    local requested_name="${1-}"
    ensure_docker
    require_command git

    umask 077
    mkdir -p "$RUNTIME_ROOT" "$LOCK_ROOT/worktrees"
    chmod 700 "$RUNTIME_ROOT" "$LOCK_ROOT" "$LOCK_ROOT/worktrees"

    local instance_id
    if [[ -n "$requested_name" ]]; then
        instance_id="$(normalize_instance "$requested_name")"
    else
        local attempt
        for attempt in {1..10}; do
            instance_id="noms-$(random_hex 8)"
            if [[ ! -e "$RUNTIME_ROOT/$instance_id" ]] &&
                [[ -z "$(project_container_ids "$instance_id")" ]]; then
                break
            fi
            instance_id=""
        done
        if [[ -z "$instance_id" ]]; then
            error "Could not allocate a unique Noms instance ID."
            return 1
        fi
    fi

    local runtime_dir="$RUNTIME_ROOT/$instance_id"
    if ! mkdir "$runtime_dir" 2>/dev/null; then
        error "Instance '$instance_id' already exists. Inspect it with 'just dev-status instance=${instance_id#noms-}' or remove it with 'just down instance=${instance_id#noms-}'."
        return 1
    fi
    chmod 700 "$runtime_dir"

    if [[ -n "$(project_container_ids "$instance_id")" ]]; then
        error "Compose project '$instance_id' already exists; refusing to attach to it."
        rmdir "$runtime_dir"
        return 1
    fi

    local worktree
    worktree="$(canonical_worktree)"
    local worktree_lock_hash
    worktree_lock_hash="$(worktree_hash "$worktree")"
    local worktree_lock="$LOCK_ROOT/worktrees/$worktree_lock_hash"

    if ! mkdir "$worktree_lock" 2>/dev/null; then
        local owner_file="$worktree_lock/owner"
        local owner_pid=""
        local owner_instance=""
        if [[ -f "$owner_file" ]]; then
            owner_pid="$(metadata_value "$owner_file" PID)"
            owner_instance="$(metadata_value "$owner_file" INSTANCE_ID)"
        fi
        if [[ "$owner_pid" =~ ^[0-9]+$ ]] && kill -0 "$owner_pid" 2>/dev/null; then
            error "Worktree '$worktree' is already used by active instance '$owner_instance' (PID $owner_pid). Create a separate Git worktree for another concurrent session."
            rmdir "$runtime_dir"
            return 1
        fi

        rm -rf -- "$worktree_lock"
        if ! mkdir "$worktree_lock" 2>/dev/null; then
            error "Another process claimed worktree '$worktree'. Create a separate Git worktree for concurrent development."
            rmdir "$runtime_dir"
            return 1
        fi
    fi

    local owner_file="$worktree_lock/owner"
    {
        printf 'INSTANCE_ID=%s\n' "$instance_id"
        printf 'PID=%s\n' "$$"
        printf 'WORKTREE=%s\n' "$worktree"
    } >"$owner_file"
    chmod 600 "$owner_file"

    local runtime_environment_file="$runtime_dir/instance.env"
    local metadata_file="$runtime_dir/metadata"
    local compose_log="$runtime_dir/compose.log"
    local instance_token
    instance_token="$(random_hex 16)"
    local session_secret
    session_secret="$(random_hex 32)"
    local session_cookie_name="noms_session_${instance_id#noms-}"
    session_cookie_name="${session_cookie_name//-/_}"
    local created_at
    created_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"

    : >"$compose_log"
    # Compose reads env_file values while creating containers. Valid loopback
    # placeholders keep the exact port-owning containers alive long enough for
    # Docker to allocate ports; the final values recreate affected services.
    write_runtime_environment "$runtime_environment_file" "$instance_id" \
        "$session_cookie_name" "$session_secret" 1 1 1 1
    chmod 600 "$compose_log"
    printf '%s\n' "$worktree_lock_hash" >"$runtime_dir/worktree.lock"
    write_metadata "$metadata_file" creating "$instance_id" "$instance_token" \
        "$worktree" "$$" "$created_at"

    local application_port=""
    local postgres_port=""
    local minio_port=""
    local minio_console_port=""
    local mock_oauth_port=""

    NOMS_UP_PROJECT_CREATED=false
    NOMS_UP_NORMAL_SHUTDOWN=false
    NOMS_UP_INSTANCE_ID="$instance_id"
    NOMS_UP_RUNTIME_DIR="$runtime_dir"
    NOMS_UP_WORKTREE="$worktree"
    NOMS_UP_WORKTREE_LOCK="$worktree_lock"
    NOMS_UP_INSTANCE_TOKEN="$instance_token"
    NOMS_UP_METADATA_FILE="$metadata_file"
    NOMS_UP_COMPOSE_LOG="$compose_log"
    NOMS_UP_CREATED_AT="$created_at"
    NOMS_UP_SESSION_COOKIE_NAME="$session_cookie_name"
    NOMS_UP_APPLICATION_PORT=""
    NOMS_UP_POSTGRES_PORT=""
    NOMS_UP_MINIO_PORT=""
    NOMS_UP_MINIO_CONSOLE_PORT=""
    NOMS_UP_MOCK_OAUTH_PORT=""
    NOMS_UP_STARTUP_SLOT=""

    # Cleanup state is global because Bash may jump directly to EXIT on an
    # errexit failure, after up_instance's local scope has been discarded.
    trap 'cleanup_up_instance "$?"' EXIT
    trap handle_up_signal INT TERM

    echo "Creating isolated Noms instance: $instance_id"
    # Compose may create some project resources before a build/create failure,
    # so cleanup owns the exact project from the moment creation begins.
    NOMS_UP_PROJECT_CREATED=true
    if ! compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" \
        create --build 2>&1 | tee -a "$compose_log"; then
        error "Container creation failed for '$instance_id'."
        return 1
    fi

    # Compose v5 allocates dynamic host ports when a container starts, not
    # when it is created. Start the exact created container IDs so dependency
    # services cannot observe the temporary runtime environment.
    start_created_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" app
    application_port="$(port_for_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" app 8080)"
    NOMS_UP_APPLICATION_PORT="$application_port"
    stop_created_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" app

    start_created_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" postgres
    start_created_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" minio
    start_created_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" mock-oauth
    postgres_port="$(port_for_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" postgres 5432)"
    minio_port="$(port_for_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" minio 9000)"
    minio_console_port="$(port_for_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" minio 9001)"
    mock_oauth_port="$(port_for_service "$instance_id" "$runtime_dir" "$worktree" "$instance_token" mock-oauth 8080)"
    NOMS_UP_POSTGRES_PORT="$postgres_port"
    NOMS_UP_MINIO_PORT="$minio_port"
    NOMS_UP_MINIO_CONSOLE_PORT="$minio_console_port"
    NOMS_UP_MOCK_OAUTH_PORT="$mock_oauth_port"

    write_runtime_environment "$runtime_environment_file" "$instance_id" \
        "$session_cookie_name" "$session_secret" "$application_port" \
        "$minio_port" "$mock_oauth_port"
    write_metadata "$metadata_file" starting "$instance_id" "$instance_token" \
        "$worktree" "$$" "$created_at" "$application_port" "$postgres_port" \
        "$minio_port" "$minio_console_port" "$mock_oauth_port" "$session_cookie_name"

    echo "Starting isolated services for $instance_id..."
    acquire_startup_slot "$instance_id" "$instance_token"
    if ! compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" \
        up --detach --wait --wait-timeout "${NOMS_STARTUP_TIMEOUT:-900}" \
        2>&1 | tee -a "$compose_log"; then
        error "Health verification or schema initialization failed for '$instance_id'."
        return 1
    fi
    release_startup_slot

    write_metadata "$metadata_file" ready "$instance_id" "$instance_token" \
        "$worktree" "$$" "$created_at" "$application_port" "$postgres_port" \
        "$minio_port" "$minio_console_port" "$mock_oauth_port" "$session_cookie_name"

    echo
    echo "Noms instance: $instance_id"
    echo "Application:   http://127.0.0.1:$application_port"
    echo "MinIO media:   http://127.0.0.1:$minio_port/noms-media"
    echo "MinIO console: http://127.0.0.1:$minio_console_port"
    echo "Mock OAuth:    http://127.0.0.1:$mock_oauth_port"
    echo "PostgreSQL:    127.0.0.1:$postgres_port"
    echo "Runtime state: $runtime_dir"
    echo "Stop with Ctrl+C or: just down instance=${instance_id#noms-}"
    echo

    compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" \
        logs --follow --no-color app 2>&1 | tee -a "$compose_log"

    if [[ -d "$runtime_dir" ]] &&
        [[ -z "$(compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" ps --status running --quiet app)" ]]; then
        error "The application container for '$instance_id' stopped unexpectedly."
        return 1
    fi
}

status_instance() {
    local requested_name="${1-}"
    local instance_id
    instance_id="$(normalize_instance "$requested_name")"
    ensure_docker

    local runtime_dir="$RUNTIME_ROOT/$instance_id"
    local metadata_file="$runtime_dir/metadata"
    if [[ ! -f "$metadata_file" ]]; then
        error "No metadata found for '$instance_id'."
        return 1
    fi
    if [[ "$(metadata_value "$metadata_file" INSTANCE_ID)" != "$instance_id" ]] ||
        [[ "$(metadata_value "$metadata_file" PROJECT_NAME)" != "$instance_id" ]]; then
        error "Metadata identity mismatch for '$instance_id'."
        return 1
    fi
    local token
    token="$(metadata_value "$metadata_file" INSTANCE_TOKEN)"
    validate_project_ownership "$instance_id" "$token" || {
        error "No active containers found for '$instance_id'."
        return 1
    }

    echo "Instance: $(metadata_value "$metadata_file" INSTANCE_ID)"
    echo "State:    $(metadata_value "$metadata_file" STATE)"
    echo "Worktree: $(metadata_value "$metadata_file" WORKTREE)"
    echo "App:      $(metadata_value "$metadata_file" APP_URL)"
    compose_command "$instance_id" "$runtime_dir" "$REPOSITORY_ROOT" "$token" ps --all
}

logs_instance() {
    local requested_name="${1-}"
    local instance_id
    instance_id="$(normalize_instance "$requested_name")"
    ensure_docker

    local runtime_dir="$RUNTIME_ROOT/$instance_id"
    local metadata_file="$runtime_dir/metadata"
    if [[ ! -f "$metadata_file" ]] ||
        [[ "$(metadata_value "$metadata_file" INSTANCE_ID)" != "$instance_id" ]]; then
        error "No valid metadata found for '$instance_id'."
        return 1
    fi
    local token
    token="$(metadata_value "$metadata_file" INSTANCE_TOKEN)"
    validate_project_ownership "$instance_id" "$token" || {
        error "No active containers found for '$instance_id'."
        return 1
    }

    compose_command "$instance_id" "$runtime_dir" "$REPOSITORY_ROOT" "$token" \
        logs --follow --no-color
}

list_instances() {
    ensure_docker
    printf '%-36s %-10s %-8s %s\n' INSTANCE STATE PID APPLICATION

    local listed_instances=""
    if [[ -d "$RUNTIME_ROOT" ]]; then
        local runtime_dir
        while IFS= read -r runtime_dir; do
            local candidate="${runtime_dir##*/}"
            [[ "$candidate" == "locks" ]] && continue
            normalize_instance "$candidate" >/dev/null 2>&1 || continue
            local metadata_file="$runtime_dir/metadata"
            [[ -f "$metadata_file" ]] || continue
            [[ "$(metadata_value "$metadata_file" INSTANCE_ID)" == "$candidate" ]] || continue
            printf '%-36s %-10s %-8s %s\n' \
                "$candidate" \
                "$(metadata_value "$metadata_file" STATE)" \
                "$(metadata_value "$metadata_file" PID)" \
                "$(metadata_value "$metadata_file" APP_URL)"
            listed_instances="${listed_instances}${candidate}"$'\n'
        done < <(find "$RUNTIME_ROOT" -mindepth 1 -maxdepth 1 -type d -print | sort)
    fi

    local docker_instance
    while IFS= read -r docker_instance; do
        [[ -n "$docker_instance" ]] || continue
        if ! grep -Fxq "$docker_instance" <<<"$listed_instances"; then
            printf '%-36s %-10s %-8s %s\n' "$docker_instance" orphaned "-" "-"
        fi
    done < <(
        docker ps --all \
            --filter "label=com.noms.dev-environment=true" \
            --format '{{.Label "com.noms.dev-instance"}}' |
            sort -u
    )
}

main() {
    local command_name="${1-}"
    shift || true

    case "$command_name" in
        normalize-instance)
            [[ "$#" -eq 1 ]] || {
                usage
                return 1
            }
            normalize_instance "$1"
            ;;
        up)
            [[ "$#" -le 1 ]] || {
                usage
                return 1
            }
            up_instance "${1-}"
            ;;
        down)
            [[ "$#" -eq 1 ]] || {
                usage
                return 1
            }
            down_instance "$1"
            ;;
        status)
            [[ "$#" -eq 1 ]] || {
                usage
                return 1
            }
            status_instance "$1"
            ;;
        logs)
            [[ "$#" -eq 1 ]] || {
                usage
                return 1
            }
            logs_instance "$1"
            ;;
        instances)
            [[ "$#" -eq 0 ]] || {
                usage
                return 1
            }
            list_instances
            ;;
        *)
            usage
            return 1
            ;;
    esac
}

main "$@"
