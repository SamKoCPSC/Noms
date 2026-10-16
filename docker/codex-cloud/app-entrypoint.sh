#!/usr/bin/env bash
set -euo pipefail
export DX_HOME="${DX_HOME:-/noms-dioxus/dx-home}"
for entry in esbuild:esbuild-0.27.3 wasm-bindgen:wasm-bindgen-0.2.121 tailwindcss:tailwindcss-v4.1.5; do
    binary="${entry%%:*}"
    directory="${entry#*:}"
    mkdir -p "$DX_HOME/tools/$directory"
    install -m 755 "/run/noms-runtime-tools/$binary" "$DX_HOME/tools/$directory/$binary"
done
exec /bin/bash /workspace/docker/development/app-entrypoint.sh
