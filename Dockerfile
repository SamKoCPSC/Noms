FROM rust:slim

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

CMD ["./target/release/noms"]
