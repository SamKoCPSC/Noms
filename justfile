# Development commands

# Start the Dioxus Fullstack dev server (hot reload + SSR)
dev:
	dx serve --platform fullstack

# Start local services (Postgres, MinIO, Mock OAuth)
services-up:
	docker compose up -d

# Stop local services
services-down:
	docker compose down

# Run database migrations locally
migrate:
	sqlx migrate run

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
