FROM rust:slim

RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

RUN cargo binstall dioxus-cli -y 2>/dev/null || cargo install dioxus-cli
RUN dx bundle --platform web --release

FROM debian:bookworm-slim
COPY --from=0 /usr/src/app/target/dx/noms/release/web/ /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT ["/usr/local/app/server"]
