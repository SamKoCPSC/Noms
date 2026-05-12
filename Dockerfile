FROM rust:slim

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release --features server

CMD ["./target/release/noms"]
