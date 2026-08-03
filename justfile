set shell := ["bash", "-cu"]

# NOMS — Local development commands
# `just up` only requires Docker; app and infrastructure tooling run in the
# invocation's isolated Compose project.

# Start a disposable isolated environment. Omit instance for an automatic ID.
up instance="":
	@./scripts/dev-environment.sh up "{{instance}}"

# Tear down exactly one isolated environment.
down instance:
	@./scripts/dev-environment.sh down "{{instance}}"

# Show state and containers for exactly one isolated environment.
dev-status instance:
	@./scripts/dev-environment.sh status "{{instance}}"

# Follow logs for exactly one isolated environment.
dev-logs instance:
	@./scripts/dev-environment.sh logs "{{instance}}"

# List active, failed, and abandoned isolated environments.
dev-instances:
	@./scripts/dev-environment.sh instances

# Run the Docker-backed two-worktree concurrency test.
isolated-dev-test:
	@bash scripts/tests/test-isolated-dev-integration.sh

# Run fast configuration checks for the isolated development environment.
isolated-dev-config-test:
	@bash scripts/tests/test-isolated-dev-config.sh

# Apply database extensions only (pgcrypto, pg_cron, etc.)
migrate-extensions:
	@source .env.local && psql "$DATABASE_URL" -f migrations/extensions.sql

# Apply the declarative database schema using the extension-enabled target for planning.
# Note: `just up` applies the schema but deliberately does not rewrite `.sqlx`.
# After changing schema or compile-time queries, run `just sqlx-prepare` explicitly.
migrate:
	@source .env.local && pgschema apply --host "$PGHOST" --port "$PGPORT" --db "$PGDATABASE" --user "$PGUSER" --password "$PGPASSWORD" --plan-host "$PGHOST" --plan-port "$PGPORT" --plan-db "$PGDATABASE" --plan-user "$PGUSER" --plan-password "$PGPASSWORD" --file migrations/schema.sql --schema public --auto-approve

# Regenerate the .sqlx/ offline query cache (requires a running DB with current schema)
sqlx-prepare:
	@source .env.local && DATABASE_URL="$DATABASE_URL" cargo sqlx prepare -- --features server

# Preview what would change (dry run)
migrate-plan:
	@source .env.local && pgschema plan --host "$PGHOST" --port "$PGPORT" --db "$PGDATABASE" --user "$PGUSER" --password "$PGPASSWORD" --plan-host "$PGHOST" --plan-port "$PGPORT" --plan-db "$PGDATABASE" --plan-user "$PGUSER" --plan-password "$PGPASSWORD" --file migrations/schema.sql --schema public --output-human stdout

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
	SQLX_OFFLINE=true cargo clippy --features server -- -D warnings

# Check compilation (both targets)
check:
	cargo check --target wasm32-unknown-unknown
	SQLX_OFFLINE=true cargo check --features server

# Run tests (uses SQLx offline mode so no live DB is needed at compile time)
test:
	SQLX_OFFLINE=true cargo test --features server

# Run CI gate script tests
schema-check-test:
	@bash scripts/tests/test-check-schema-plan.sh
	@bash scripts/tests/test-pgschema-workflow.sh

# Run all checks and tests (mirrors what CI does)
ci: fmt check lint test schema-check-test
	@echo ""
	@echo "✅ All checks passed."
