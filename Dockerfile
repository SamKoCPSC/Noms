FROM rust:1.85-slim

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

CMD ["./target/release/noms"]
