#!/usr/bin/env bash
set -euo pipefail

SOURCE_ROOT="$(git rev-parse --show-toplevel)"
COMMON_WORKTREE_ROOT="$(cd "$(git rev-parse --git-common-dir)/.." && pwd -P)/.worktrees"
mkdir -p "$COMMON_WORKTREE_ROOT"
TEST_ROOT="$(mktemp -d "$COMMON_WORKTREE_ROOT/noms-isolation-test.XXXXXX")"
WORKTREE_A="$TEST_ROOT/worktree-a"
WORKTREE_B="$TEST_ROOT/worktree-b"
RUNTIME_ROOT="$TEST_ROOT/runtime"
INSTANCE_A="noms-integration-a"
INSTANCE_B="noms-integration-b"
UP_PID_A=""
UP_PID_B=""

compose_for() {
    local worktree="$1"
    local instance="$2"
    local runtime_dir="$RUNTIME_ROOT/$instance"
    NOMS_INSTANCE_ID="$instance" \
    NOMS_PROJECT_NAME="$instance" \
    NOMS_RUNTIME_DIR="$runtime_dir" \
    NOMS_WORKTREE="$worktree" \
        docker compose \
            --project-name "$instance" \
            --file "$worktree/docker-compose.yml" \
            "${@:3}"
}

cleanup() {
    set +e
    if [[ -d "$WORKTREE_A" ]]; then
        (cd "$WORKTREE_A" && NOMS_RUNTIME_ROOT="$RUNTIME_ROOT" just down instance="$INSTANCE_A") >/dev/null 2>&1
    fi
    if [[ -d "$WORKTREE_B" ]]; then
        (cd "$WORKTREE_B" && NOMS_RUNTIME_ROOT="$RUNTIME_ROOT" just down instance="$INSTANCE_B") >/dev/null 2>&1
    fi
    if [[ -n "$UP_PID_A" ]]; then
        kill -TERM "$UP_PID_A" 2>/dev/null
        wait "$UP_PID_A" 2>/dev/null
    fi
    if [[ -n "$UP_PID_B" ]]; then
        kill -TERM "$UP_PID_B" 2>/dev/null
        wait "$UP_PID_B" 2>/dev/null
    fi
    git -C "$SOURCE_ROOT" worktree remove --force "$WORKTREE_A" >/dev/null 2>&1
    git -C "$SOURCE_ROOT" worktree remove --force "$WORKTREE_B" >/dev/null 2>&1
    rm -rf "$TEST_ROOT"
}
trap cleanup EXIT

copy_uncommitted_change() {
    local destination="$1"
    local patch_file="$TEST_ROOT/current.patch"

    if [[ ! -f "$patch_file" ]]; then
        git -C "$SOURCE_ROOT" diff --binary HEAD >"$patch_file"
    fi
    if [[ -s "$patch_file" ]]; then
        git -C "$destination" apply "$patch_file"
    fi

    while IFS= read -r -d '' untracked_file; do
        mkdir -p "$destination/$(dirname "$untracked_file")"
        cp "$SOURCE_ROOT/$untracked_file" "$destination/$untracked_file"
        if [[ -x "$SOURCE_ROOT/$untracked_file" ]]; then
            chmod +x "$destination/$untracked_file"
        fi
    done < <(git -C "$SOURCE_ROOT" ls-files --others --exclude-standard -z)
}

metadata_value() {
    local instance="$1"
    local key="$2"
    sed -n "s/^${key}=//p" "$RUNTIME_ROOT/$instance/metadata"
}

wait_for_metadata() {
    local instance="$1"
    local log_file="$2"
    local process_id="$3"
    local attempts=1200
    while ((attempts > 0)); do
        if [[ -s "$RUNTIME_ROOT/$instance/metadata" ]] &&
            [[ "$(metadata_value "$instance" STATE)" == "ready" ]]; then
            return 0
        fi
        if grep -q '^ERROR:' "$log_file" 2>/dev/null; then
            return 1
        fi
        if ! kill -0 "$process_id" 2>/dev/null; then
            return 1
        fi
        sleep 2
        attempts=$((attempts - 1))
    done
    return 1
}

assert_distinct() {
    local description="$1"
    local first="$2"
    local second="$3"
    if [[ -z "$first" || -z "$second" || "$first" == "$second" ]]; then
        echo "FAIL: $description ('$first' vs '$second')" >&2
        exit 1
    fi
    echo "PASS: $description"
}

assert_equal() {
    local description="$1"
    local expected="$2"
    local actual="$3"
    if [[ -z "$expected" || "$expected" != "$actual" ]]; then
        echo "FAIL: $description (expected '$expected', received '$actual')" >&2
        exit 1
    fi
    echo "PASS: $description"
}

complete_mock_oauth_login() {
    local description="$1"
    local application_url="$2"
    local cookie_name="$3"
    local cookie_jar="$4"
    local authorization_url
    local final_url

    : >"$cookie_jar"
    authorization_url="$(
        curl --fail --silent --show-error \
            --output /dev/null \
            --write-out '%{redirect_url}' \
            "$application_url/auth/google/start?redirect_uri=%2Fdashboard"
    )"
    if [[ "$authorization_url" != http://127.0.0.1:*/google/authorize\?* ]]; then
        echo "FAIL: $description did not reach its Mock OAuth authorization endpoint" >&2
        exit 1
    fi

    final_url="$(
        curl --fail --silent --show-error \
            --location \
            --max-redirs 10 \
            --cookie "$cookie_jar" \
            --cookie-jar "$cookie_jar" \
            --data-urlencode "username=$cookie_name" \
            --data-urlencode "claims={\"email\":\"$cookie_name@example.test\",\"name\":\"$description\"}" \
            --output /dev/null \
            --write-out '%{url_effective}' \
            "$authorization_url"
    )"
    assert_equal "$description returns to its own application" \
        "$application_url/dashboard" "$final_url"

    if ! awk -v cookie_name="$cookie_name" \
        '$6 == cookie_name && length($7) > 0 { found = 1 } END { exit !found }' \
        "$cookie_jar"; then
        echo "FAIL: $description did not establish cookie '$cookie_name'" >&2
        exit 1
    fi
    echo "PASS: $description establishes its instance-specific session"
}

profile_is_authenticated() {
    local application_url="$1"
    local cookie_jar="$2"
    curl --silent --show-error \
        --cookie "$cookie_jar" \
        "$application_url/api/user_profile" |
        grep -q '"is_authenticated":true'
}

dump_instance_failure() {
    local instance="$1"
    local worktree="$2"
    echo "Diagnostics for $instance:" >&2
    compose_for "$worktree" "$instance" ps --all >&2 || true
    local container_id
    while IFS= read -r container_id; do
        [[ -n "$container_id" ]] || continue
        docker inspect --format \
            '{{.Name}} exit={{.State.ExitCode}} oom_killed={{.State.OOMKilled}} error={{json .State.Error}}' \
            "$container_id" >&2 || true
    done < <(docker ps --all --quiet --filter "label=com.docker.compose.project=$instance")
    if [[ -f "$RUNTIME_ROOT/$instance/compose.log" ]]; then
        tail -n 200 "$RUNTIME_ROOT/$instance/compose.log" >&2
    fi
}

echo "Creating isolated test worktrees..."
git -C "$SOURCE_ROOT" worktree add --detach "$WORKTREE_A" HEAD
git -C "$SOURCE_ROOT" worktree add --detach "$WORKTREE_B" HEAD
copy_uncommitted_change "$WORKTREE_A"
copy_uncommitted_change "$WORKTREE_B"

echo "Starting two Noms environments concurrently..."
(
    cd "$WORKTREE_A"
    NOMS_RUNTIME_ROOT="$RUNTIME_ROOT" just up instance="$INSTANCE_A"
) >"$TEST_ROOT/instance-a.log" 2>&1 &
UP_PID_A=$!
(
    cd "$WORKTREE_B"
    NOMS_RUNTIME_ROOT="$RUNTIME_ROOT" just up instance="$INSTANCE_B"
) >"$TEST_ROOT/instance-b.log" 2>&1 &
UP_PID_B=$!

if ! wait_for_metadata "$INSTANCE_A" "$TEST_ROOT/instance-a.log" "$UP_PID_A"; then
    wait "$UP_PID_A" 2>/dev/null || true
    cat "$TEST_ROOT/instance-a.log" >&2
    if [[ -f "$RUNTIME_ROOT/$INSTANCE_A/compose.log" ]]; then
        tail -n 200 "$RUNTIME_ROOT/$INSTANCE_A/compose.log" >&2
    fi
    exit 1
fi
if ! wait_for_metadata "$INSTANCE_B" "$TEST_ROOT/instance-b.log" "$UP_PID_B"; then
    wait "$UP_PID_B" 2>/dev/null || true
    cat "$TEST_ROOT/instance-b.log" >&2
    if [[ -f "$RUNTIME_ROOT/$INSTANCE_B/compose.log" ]]; then
        tail -n 200 "$RUNTIME_ROOT/$INSTANCE_B/compose.log" >&2
    fi
    exit 1
fi

APP_URL_A="$(metadata_value "$INSTANCE_A" APP_URL)"
APP_URL_B="$(metadata_value "$INSTANCE_B" APP_URL)"
POSTGRES_PORT_A="$(metadata_value "$INSTANCE_A" POSTGRES_PORT)"
POSTGRES_PORT_B="$(metadata_value "$INSTANCE_B" POSTGRES_PORT)"
MINIO_PORT_A="$(metadata_value "$INSTANCE_A" MINIO_PORT)"
MINIO_PORT_B="$(metadata_value "$INSTANCE_B" MINIO_PORT)"
COOKIE_NAME_A="$(metadata_value "$INSTANCE_A" SESSION_COOKIE_NAME)"
COOKIE_NAME_B="$(metadata_value "$INSTANCE_B" SESSION_COOKIE_NAME)"
RUNTIME_DIR_A="$(metadata_value "$INSTANCE_A" RUNTIME_DIR)"
RUNTIME_DIR_B="$(metadata_value "$INSTANCE_B" RUNTIME_DIR)"

assert_distinct "Compose project names are isolated" "$INSTANCE_A" "$INSTANCE_B"
assert_distinct "application origins are isolated" "$APP_URL_A" "$APP_URL_B"
assert_distinct "PostgreSQL ports are isolated" "$POSTGRES_PORT_A" "$POSTGRES_PORT_B"
assert_distinct "MinIO ports are isolated" "$MINIO_PORT_A" "$MINIO_PORT_B"
assert_distinct "session cookie names are isolated" "$COOKIE_NAME_A" "$COOKIE_NAME_B"
assert_distinct "runtime directories are isolated" "$RUNTIME_DIR_A" "$RUNTIME_DIR_B"

ACTUAL_APP_PORT_A="$(compose_for "$WORKTREE_A" "$INSTANCE_A" port app 8080)"
ACTUAL_APP_PORT_A="${ACTUAL_APP_PORT_A##*:}"
ACTUAL_APP_PORT_B="$(compose_for "$WORKTREE_B" "$INSTANCE_B" port app 8080)"
ACTUAL_APP_PORT_B="${ACTUAL_APP_PORT_B##*:}"
ACTUAL_MINIO_PORT_A="$(compose_for "$WORKTREE_A" "$INSTANCE_A" port minio 9000)"
ACTUAL_MINIO_PORT_A="${ACTUAL_MINIO_PORT_A##*:}"
ACTUAL_MINIO_PORT_B="$(compose_for "$WORKTREE_B" "$INSTANCE_B" port minio 9000)"
ACTUAL_MINIO_PORT_B="${ACTUAL_MINIO_PORT_B##*:}"
assert_equal "instance A advertises its live application port" \
    "${APP_URL_A##*:}" "$ACTUAL_APP_PORT_A"
assert_equal "instance B advertises its live application port" \
    "${APP_URL_B##*:}" "$ACTUAL_APP_PORT_B"
assert_equal "instance A advertises its live MinIO port" \
    "$MINIO_PORT_A" "$ACTUAL_MINIO_PORT_A"
assert_equal "instance B advertises its live MinIO port" \
    "$MINIO_PORT_B" "$ACTUAL_MINIO_PORT_B"

CONTAINER_A="$(compose_for "$WORKTREE_A" "$INSTANCE_A" ps -q postgres)"
CONTAINER_B="$(compose_for "$WORKTREE_B" "$INSTANCE_B" ps -q postgres)"
NETWORK_A="$(docker inspect "$CONTAINER_A" --format '{{range $name, $_ := .NetworkSettings.Networks}}{{$name}}{{end}}')"
NETWORK_B="$(docker inspect "$CONTAINER_B" --format '{{range $name, $_ := .NetworkSettings.Networks}}{{$name}}{{end}}')"
VOLUME_A="$(docker inspect "$CONTAINER_A" --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}')"
VOLUME_B="$(docker inspect "$CONTAINER_B" --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}')"
assert_distinct "PostgreSQL containers are isolated" "$CONTAINER_A" "$CONTAINER_B"
assert_distinct "Compose networks are isolated" "$NETWORK_A" "$NETWORK_B"
assert_distinct "PostgreSQL volumes are isolated" "$VOLUME_A" "$VOLUME_B"

APP_CONTAINER_A="$(compose_for "$WORKTREE_A" "$INSTANCE_A" ps -q app)"
APP_CONTAINER_B="$(compose_for "$WORKTREE_B" "$INSTANCE_B" ps -q app)"
for mount_path in /noms-cargo /noms-target /noms-dioxus; do
    APP_VOLUME_A="$(docker inspect "$APP_CONTAINER_A" \
        --format "{{range .Mounts}}{{if eq .Destination \"$mount_path\"}}{{.Name}}{{end}}{{end}}")"
    APP_VOLUME_B="$(docker inspect "$APP_CONTAINER_B" \
        --format "{{range .Mounts}}{{if eq .Destination \"$mount_path\"}}{{.Name}}{{end}}{{end}}")"
    assert_distinct "application volume $mount_path is isolated" "$APP_VOLUME_A" "$APP_VOLUME_B"
done

MINIO_CONTAINER_A="$(compose_for "$WORKTREE_A" "$INSTANCE_A" ps -q minio)"
MINIO_CONTAINER_B="$(compose_for "$WORKTREE_B" "$INSTANCE_B" ps -q minio)"
MINIO_VOLUME_A="$(docker inspect "$MINIO_CONTAINER_A" \
    --format '{{range .Mounts}}{{if eq .Destination "/data"}}{{.Name}}{{end}}{{end}}')"
MINIO_VOLUME_B="$(docker inspect "$MINIO_CONTAINER_B" \
    --format '{{range .Mounts}}{{if eq .Destination "/data"}}{{.Name}}{{end}}{{end}}')"
assert_distinct "MinIO volumes are isolated" "$MINIO_VOLUME_A" "$MINIO_VOLUME_B"

compose_for "$WORKTREE_A" "$INSTANCE_A" exec -T postgres \
    psql -U noms -d noms -c 'CREATE TABLE isolation_probe_a (id integer PRIMARY KEY); INSERT INTO isolation_probe_a VALUES (1);'
compose_for "$WORKTREE_B" "$INSTANCE_B" exec -T postgres \
    psql -U noms -d noms -c 'CREATE TABLE isolation_probe_b (id integer PRIMARY KEY); INSERT INTO isolation_probe_b VALUES (1);'
if compose_for "$WORKTREE_B" "$INSTANCE_B" exec -T postgres \
    psql -U noms -d noms -Atqc "SELECT to_regclass('public.isolation_probe_a')" |
    grep -q isolation_probe_a; then
    echo "FAIL: database state leaked from instance A to B" >&2
    exit 1
fi
echo "PASS: database state is isolated"

printf 'instance-a' | compose_for "$WORKTREE_A" "$INSTANCE_A" exec -T minio \
    sh -c 'mc alias set isolation-probe http://localhost:9000 noms-local-key noms-local-secret >/dev/null &&
        mc pipe isolation-probe/noms-media/isolation-a.txt'
if compose_for "$WORKTREE_B" "$INSTANCE_B" exec -T minio \
    sh -c 'mc alias set isolation-probe http://localhost:9000 noms-local-key noms-local-secret >/dev/null &&
        mc stat isolation-probe/noms-media/isolation-a.txt' >/dev/null 2>&1; then
    echo "FAIL: MinIO state leaked from instance A to B" >&2
    exit 1
fi
echo "PASS: MinIO state is isolated"

if ! curl --fail --silent --show-error "$APP_URL_A/" >/dev/null; then
    dump_instance_failure "$INSTANCE_A" "$WORKTREE_A"
    exit 1
fi
if ! curl --fail --silent --show-error "$APP_URL_B/" >/dev/null; then
    dump_instance_failure "$INSTANCE_B" "$WORKTREE_B"
    exit 1
fi

COOKIE_JAR_A="$TEST_ROOT/instance-a.cookies"
COOKIE_JAR_B="$TEST_ROOT/instance-b.cookies"
complete_mock_oauth_login "instance A Mock OAuth login" \
    "$APP_URL_A" "$COOKIE_NAME_A" "$COOKIE_JAR_A"
complete_mock_oauth_login "instance B Mock OAuth login" \
    "$APP_URL_B" "$COOKIE_NAME_B" "$COOKIE_JAR_B"

if profile_is_authenticated "$APP_URL_A" "$COOKIE_JAR_A" &&
    profile_is_authenticated "$APP_URL_B" "$COOKIE_JAR_B"; then
    echo "PASS: each instance accepts its own authenticated session"
else
    echo "FAIL: an instance did not accept its own authenticated session" >&2
    exit 1
fi

if profile_is_authenticated "$APP_URL_B" "$COOKIE_JAR_A"; then
    echo "FAIL: instance A's session was accepted by instance B" >&2
    exit 1
fi
echo "PASS: sessions are not interchangeable between instances"

curl --fail --silent --show-error \
    --location \
    --cookie "$COOKIE_JAR_A" \
    --cookie-jar "$COOKIE_JAR_A" \
    "$APP_URL_A/auth/logout?redirect_uri=%2F" >/dev/null
if profile_is_authenticated "$APP_URL_A" "$COOKIE_JAR_A"; then
    echo "FAIL: signing out of instance A left its session authenticated" >&2
    exit 1
fi
if ! profile_is_authenticated "$APP_URL_B" "$COOKIE_JAR_B"; then
    echo "FAIL: signing out of instance A invalidated instance B" >&2
    exit 1
fi
echo "PASS: signing out of instance A leaves instance B authenticated"

echo "Stopping instance A while instance B remains active..."
(
    cd "$WORKTREE_A"
    NOMS_RUNTIME_ROOT="$RUNTIME_ROOT" just down instance="$INSTANCE_A"
)
wait "$UP_PID_A" 2>/dev/null || true
UP_PID_A=""
curl --fail --silent --show-error "$APP_URL_B/" >/dev/null
compose_for "$WORKTREE_B" "$INSTANCE_B" exec -T postgres \
    psql -U noms -d noms -Atqc 'SELECT count(*) FROM isolation_probe_b' |
    grep -qx 1
echo "PASS: stopping instance A leaves instance B healthy"

echo "Isolated development integration test passed."
