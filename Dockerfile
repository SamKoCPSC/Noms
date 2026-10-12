# === Stage 1: Prepare dependency recipe ===
FROM rust:1-slim-bookworm@sha256:2775a09d208ff0d7c1f50490c45b62db929e87ba1dcbc3f2132ac71a704bcdd3 AS planner
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --version 0.1.78 --locked
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock clippy.toml ./
COPY src/ ./src/
RUN cargo chef prepare --recipe-path recipe.json

# === Stage 2: Compile dependencies (cached across builds) ===
FROM rust:1-slim-bookworm@sha256:2775a09d208ff0d7c1f50490c45b62db929e87ba1dcbc3f2132ac71a704bcdd3 AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --version 0.1.78 --locked
RUN cargo install dioxus-cli@0.7.9 --locked
WORKDIR /usr/src/app
COPY --from=planner /usr/src/app/recipe.json recipe.json
# Bound application compilation on shared builders; Dioxus also builds its
# client and server targets sequentially below to limit peak memory usage.
ENV CARGO_BUILD_JOBS=2 \
    SQLX_OFFLINE=true
RUN cargo chef cook --release --recipe-path recipe.json

# === Stage 2b: Build pgschema ===
FROM golang:1.24-bookworm@sha256:1a6d4452c65dea36aac2e2d606b01b4a029ec90cc1ae53890540ce6173ea77ac AS pgschema-builder
RUN go install github.com/pgplex/pgschema@v1.12.1

# === Stage 3: Release bundle ===
# Formatting, linting and tests run in GitHub CI before the publish-image job.
# Railway pulls the published release image instead of compiling it again.
# Must switch back to the Rust builder context — the pgschema-builder stage
# uses a Go image which has no cargo toolchain.
FROM builder AS build-check
COPY . .
RUN dx bundle --platform web --release --force-sequential true

# === Stage 4: Runtime ===
FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241
RUN apt-get update && apt-get install -y curl postgresql-client perl && rm -rf /var/lib/apt/lists/* \
    && groupadd -r noms && useradd -r -g noms -d /usr/local/app -s /sbin/nologin noms \
    && mkdir -p /usr/local/bin

COPY --from=pgschema-builder /go/bin/pgschema /usr/local/bin/pgschema
COPY --from=build-check /usr/src/app/target/dx/noms/release/web/ /usr/local/app
COPY --from=build-check /usr/src/app/migrations/ /usr/local/app/migrations/
COPY entrypoint.sh /usr/local/app/entrypoint.sh
RUN chmod 0555 /usr/local/app /usr/local/app/entrypoint.sh \
    && chmod -R go-w /usr/local/app

USER noms
WORKDIR /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -f http://localhost:8080/ || exit 1

ENTRYPOINT ["/usr/local/app/entrypoint.sh"]
