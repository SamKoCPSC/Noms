set shell := ["bash", "-cu"]

# NOMS — Local development commands
# Prerequisites: docker, just, cargo/dx, pgschema, psql (postgresql client)

# One-command local startup: Docker services + extensions + schema + dev server
up:
	# Ensure Docker is running (auto-start if possible for the platform)
	@if ! docker info > /dev/null 2>&1; then \
		case "$(uname -s)" in \
			Linux) \
				if command -v rancher-desktop > /dev/null 2>&1; then \
					echo "Starting Rancher Desktop..."; rancher-desktop & \
				else \
					echo "Error: Docker is not running. Please start your Docker engine and try again."; exit 1; \
				fi \
				;; \
			Darwin) \
				echo "Starting Docker Desktop..."; open -a Docker & \
				;; \
			MINGW*|MSYS*|CYGWIN*) \
				echo "Starting Docker Desktop..."; cmd.exe /c start Docker & \
				;; \
			*) \
				echo "Error: Docker is not running. Please start Docker and try again."; exit 1 \
				;; \
		esac; \
	fi
	@while ! docker info > /dev/null 2>&1; do echo "Waiting for Docker to start..."; sleep 2; done
	# Start infrastructure services in background
	docker compose up -d
	# Wait for all services to become healthy
	@until [ "$(docker inspect $(docker compose ps -q postgres) | jq -r '.[0].State.Health.Status')" == "healthy" ]; do echo "Waiting for postgres to be healthy..."; sleep 2; done
	@until [ "$(docker inspect $(docker compose ps -q minio) | jq -r '.[0].State.Health.Status')" == "healthy" ]; do echo "Waiting for minio to be healthy..."; sleep 2; done
	@until [ "$(docker inspect $(docker compose ps -q mock-oauth) | jq -r '.[0].State.Health.Status')" == "healthy" ]; do echo "Waiting for mock-oauth to be healthy..."; sleep 2; done
	# Apply database extensions and schema
	@source .env.local && psql "$DATABASE_URL" -f migrations/extensions.sql && pgschema apply --host "$PGHOST" --port "$PGPORT" --db "$PGDATABASE" --user "$PGUSER" --password "$PGPASSWORD" --file migrations/schema.sql --schema public --auto-approve
	# Launch dev server; clean up Docker on Ctrl+C
	@trap 'docker compose down --remove-orphans' INT TERM EXIT; \
	dx serve --platform web; \
	trap - INT TERM EXIT; \
	docker compose down --remove-orphans

# One-command local teardown
down:
	docker compose down --remove-orphans

# Apply database extensions only (pgcrypto, pg_cron, etc.)
migrate-extensions:
	@source .env.local && psql "$DATABASE_URL" -f migrations/extensions.sql

# Apply the declarative database schema (run after modifying migrations/schema.sql)
migrate:
	@source .env.local && pgschema apply --host "$PGHOST" --port "$PGPORT" --db "$PGDATABASE" --user "$PGUSER" --password "$PGPASSWORD" --file migrations/schema.sql --schema public --auto-approve

# Preview what would change (dry run)
migrate-plan:
	@source .env.local && pgschema plan --host "$PGHOST" --port "$PGPORT" --db "$PGDATABASE" --user "$PGUSER" --password "$PGPASSWORD" --file migrations/schema.sql --schema public --output-human stdout

# Apply extensions + schema in one go
migrate-full: migrate-extensions migrate

# Start the Dioxus Fullstack dev server (hot reload + SSR)
dev:
	dx serve --platform web

# Format code
fmt:
	cargo fmt

# Run fmt + clippy (both targets)
lint:
	cargo fmt
	cargo clippy --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --features server -- -D warnings

# Check compilation (both targets)
check:
	cargo check --target wasm32-unknown-unknown
	cargo check --features server

# Run tests
test:
	cargo test --features server

# Run CI gate script tests
schema-check-test:
	@bash scripts/tests/test-check-schema-plan.sh

# Run all checks and tests (mirrors what CI does)
ci: fmt check lint test schema-check-test
	@echo ""
	@echo "✅ All checks passed."