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

# Run tests (uses SQLx offline mode so no live DB is needed at compile time).
# Cap concurrency because database tests each start an isolated PostgreSQL
# process; matching the host's full CPU count can exhaust startup resources.
test:
	SQLX_OFFLINE=true cargo test --features server -- --test-threads=8

# Run CI gate script tests
schema-check-test:
	@bash scripts/tests/test-check-schema-plan.sh
	@bash scripts/tests/test-pgschema-workflow.sh

# Run security policy regression tests without network access.
security-policy-test:
	@bash scripts/tests/test-security-policy.sh

# Audit dependencies and deployment artifacts using pinned policy.
security-check:
	@./scripts/security-audit.sh
	@./scripts/check-tracked-secrets.sh
	@./scripts/check-deployment-artifacts.sh

# Validate the NOMS-016 corpus analyzer and synthetic import contract fixtures.
recipe-import-fixture-test:
	@node --test scripts/tests/test-analyze-recipe-corpus.mjs
	@node --test scripts/tests/test-validate-recipe-import-fixtures.mjs

# Full suite stays red until every incremental parser slice is implemented.
recipe-import-parser-acceptance-test:
	@node --test scripts/tests/test-recipe-import-parser-acceptance.mjs

# First parser slice: clean HTML plus generic plain-text fallback.
recipe-import-parser-basic-test:
	@node --test --test-name-pattern='basic-visible-recipe|plain-text-fallback' scripts/tests/test-recipe-import-parser-acceptance.mjs

# Cumulative parser slices through grouped ingredient and instruction components.
recipe-import-parser-grouped-test:
	@node --test --test-name-pattern='basic-visible-recipe|grouped-components|plain-text-fallback' scripts/tests/test-recipe-import-parser-acceptance.mjs

# Cumulative parser slices through competing-region selection and quality gates.
recipe-import-parser-region-test:
	@node --test --test-name-pattern='basic-visible-recipe|grouped-components|competing-recipe-regions|collection-page-embedded-recipe|navigation-ingredient-region|missing-recipe-title|plain-text-fallback' scripts/tests/test-recipe-import-parser-acceptance.mjs

# Cumulative parser slices through active measurement variants, yield, and durations.
recipe-import-parser-variant-test:
	@node --test --test-name-pattern='basic-visible-recipe|grouped-components|competing-recipe-regions|collection-page-embedded-recipe|navigation-ingredient-region|missing-recipe-title|unit-yield-and-duration-ambiguity|plain-text-fallback' scripts/tests/test-recipe-import-parser-acceptance.mjs

# Cumulative parser slices through notes, equipment, and image references.
recipe-import-parser-media-test:
	@node --test --test-name-pattern='basic-visible-recipe|grouped-components|competing-recipe-regions|collection-page-embedded-recipe|navigation-ingredient-region|missing-recipe-title|unit-yield-and-duration-ambiguity|notes-equipment-and-media|plain-text-fallback' scripts/tests/test-recipe-import-parser-acceptance.mjs

# Cumulative parser slices through inert active content and remote-resource reporting.
recipe-import-parser-active-content-test:
	@node --test --test-name-pattern='basic-visible-recipe|grouped-components|competing-recipe-regions|collection-page-embedded-recipe|navigation-ingredient-region|missing-recipe-title|unit-yield-and-duration-ambiguity|notes-equipment-and-media|remote-content-and-controls|plain-text-fallback' scripts/tests/test-recipe-import-parser-acceptance.mjs

# Validate the privacy-preserving real-corpus parser evaluator.
recipe-import-parser-corpus-test:
	@node --test scripts/tests/test-evaluate-recipe-parser-corpus.mjs

# Validate privacy-preserving, digest-bound manual semantic review scoring.
recipe-import-parser-semantic-test:
	@node --test scripts/tests/test-evaluate-recipe-parser-semantic.mjs

# Validate source-scoped adapters and conservative generic fallback.
recipe-import-provider-adapter-test:
	@node --test scripts/tests/test-recipe-import-provider-adapters.mjs
	@node --test scripts/tests/test-recipe-import-allrecipes.mjs
	@node --test scripts/tests/test-recipe-import-food52.mjs
	@node --test scripts/tests/test-recipe-import-foodcom.mjs

# Verify browser clipboard capture, bounds, cancellation, and parser handoff.
recipe-import-bridge-test:
	@node --test scripts/tests/test-recipe-import-bridge.mjs

# Run all checks and tests (mirrors what CI does)
ci: fmt check lint test schema-check-test security-policy-test security-check recipe-import-fixture-test recipe-import-parser-active-content-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test recipe-import-provider-adapter-test recipe-import-bridge-test
	@echo ""
	@echo "✅ All checks passed."
