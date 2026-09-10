FROM golang:1.24-bookworm@sha256:1a6d4452c65dea36aac2e2d606b01b4a029ec90cc1ae53890540ce6173ea77ac AS pgschema-builder
RUN go install github.com/pgplex/pgschema@v1.12.1

FROM rust:1-bookworm@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       ca-certificates \
       curl \
       libssl-dev \
       pkg-config \
       postgresql-client \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown \
    && cargo install dioxus-cli@0.7.9 --locked \
    && cp /usr/local/cargo/bin/dx /usr/local/bin/dx

RUN ln -s \
    "$(find /usr/local/rustup -path '*/gcc-ld/ld.lld' -print -quit)" \
    /usr/local/bin/ld.lld

RUN useradd --create-home --user-group noms

COPY --from=pgschema-builder /go/bin/pgschema /usr/local/bin/pgschema

WORKDIR /workspace
