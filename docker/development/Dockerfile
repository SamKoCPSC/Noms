# Run the Go compiler natively even when the runtime is linux/amd64 on an
# Apple Silicon host. Emulated Go/GCC compilation can crash under QEMU.
FROM --platform=$BUILDPLATFORM golang:1.25.3-bookworm@sha256:4f43b271f9673eb7bd0cb3a49cc17b08d8d6ee110277e26dbacc93c43a5a7793 AS pgschema-builder
ARG TARGETOS
ARG TARGETARCH
ENV GOTOOLCHAIN=local CGO_ENABLED=0 GOMAXPROCS=2 GOOS=$TARGETOS GOARCH=$TARGETARCH
RUN go install -p 2 github.com/pgplex/pgschema@v1.12.1

# MinIO's published container images are no longer publicly available. Build
# the official sources at immutable versions; Go verifies module checksums.
# Bound compilation parallelism for small local Docker VMs.
RUN go install -p 2 github.com/minio/minio@v0.0.0-20260212201848-7aac2a2c5b7c
RUN go install -p 2 github.com/minio/mc@v0.0.0-20251106162529-77f82e18b540

# go install uses a platform subdirectory for cross-compiled binaries, but
# puts native binaries directly in /go/bin. Give the runtime stable paths.
RUN mkdir -p /out \
    && for tool in pgschema minio mc; do \
         tool_path="/go/bin/${GOOS}_${GOARCH}/$tool"; \
         [ -f "$tool_path" ] || tool_path="/go/bin/$tool"; \
         cp "$tool_path" "/out/$tool"; \
       done

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

COPY --from=pgschema-builder /out/pgschema /usr/local/bin/pgschema
COPY --from=pgschema-builder /out/minio /usr/local/bin/minio
COPY --from=pgschema-builder /out/mc /usr/local/bin/mc

WORKDIR /workspace
