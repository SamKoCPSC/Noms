FROM rust:slim

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release --no-default-features --features server

CMD ["./target/release/noms"]
