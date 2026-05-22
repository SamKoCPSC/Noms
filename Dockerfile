# === Stage 1: Prepare dependency recipe ===
FROM rust:slim AS planner
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock clippy.toml ./
COPY src/ ./src/
RUN cargo chef prepare --recipe-path recipe.json

# === Stage 2: Compile dependencies (cached across builds) ===
FROM rust:slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
RUN cargo install cargo-chef && cargo install dioxus-cli
WORKDIR /usr/src/app
COPY --from=planner /usr/src/app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# === Stage 3: Checks + final build ===
COPY . .
RUN cargo fmt --check \
    && cargo clippy --no-default-features --features server -- -D warnings \
    && cargo test --no-default-features --features server
RUN dx bundle --platform web --release

# === Stage 4: Runtime ===
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/* \
    && groupadd -r noms && useradd -r -g noms -d /usr/local/app -s /sbin/nologin noms \
    && curl -fsSL https://github.com/fmguerreiro/pgmold/releases/download/v0.34.12/pgmold-x86_64-unknown-linux-gnu.tar.gz | tar xz -C /usr/local/bin

COPY --from=builder /usr/src/app/target/dx/noms/release/web/ /usr/local/app
COPY --from=builder /usr/src/app/migrations/ /usr/local/app/migrations/
COPY entrypoint.sh /usr/local/app/entrypoint.sh
RUN chmod +x /usr/local/app/entrypoint.sh \
    && chown -R noms:noms /usr/local/app

USER noms
WORKDIR /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -f http://localhost:8080/ || exit 1

ENTRYPOINT ["/usr/local/app/entrypoint.sh"]
