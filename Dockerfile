FROM rust:slim

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

RUN cargo install dioxus-cli && dx bundle --release

CMD ["./target/release/noms"]
