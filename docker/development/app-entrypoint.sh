#!/usr/bin/env bash
set -euo pipefail

: "${NOMS_INSTANCE_ID:?Missing isolated development environment}"

if [[ "${NOMS_PORT_BOOTSTRAP:-0}" == "1" ]]; then
    trap 'exit 0' INT TERM
    while true; do
        sleep 3600 &
        wait "$!" || true
    done
fi

: "${DATABASE_URL:?Missing isolated database configuration}"
: "${BASE_URL:?Missing isolated application origin}"

cd /workspace

# Schema initialization is a Compose dependency. Load reviewed vocabulary once
# before serving; production startup never invokes the seed CLI.
cargo run --features server --bin seed-vocabulary --quiet

dx serve \
    --platform web \
    --addr 0.0.0.0 \
    --port 8080 \
    --open false \
    --interactive false \
    --force-sequential true \
    --debug-symbols false &
dx_pid=$!

terminate_app() {
    trap - INT TERM
    kill -TERM "$dx_pid" 2>/dev/null || true
    wait "$dx_pid" 2>/dev/null || true
    exit 0
}
trap terminate_app INT TERM

set +e
wait "$dx_pid"
exit_status=$?
set -e
exit "$exit_status"
