# Codex cloud development environment

This folder makes Noms reproducible in a managed Codex cloud executor with an
outbound proxy, a cloud CA, Docker-in-Docker, and limited disk space. It starts
the same isolated development stack as `just up`: app, PostgreSQL, MinIO, mock
OAuth, mock GitHub API, bucket initialization, and declarative schema setup.
It also supports **headless Chromium testing and screenshots inside the executor**.
A public URL, port forward, or remotely controlled browser is not needed for that.

This is a development environment with synthetic OAuth and local credentials,
not a public deployment. The production Dockerfile, Railway setup, shared
Dockerfiles, and ordinary `just up` workflow are unchanged. Do not publish this
mock-authentication stack as a production service.

## Before starting (agents: read this first)

1. Read the installed `cloud-environment-runtime` skill and its Docker/networking
   references if available. Call `environment_status` and check the **actual**
   bound instance/configuration version, current observations, and enforced
   policy. A published configuration draft is not proof of runtime readiness.
2. Inspect `/etc/codex/network-policy.json`. The normal package-manager preset
   is needed for Debian, Cargo, Go, GitHub release assets, npm, packagecloud, and
   container registries. This app additionally needed `apt.postgresql.org` and
   `storage.googleapis.com` (Go module download redirects). Have the user use
   the platform's reviewed configuration flow if a host is missing. Do not add
   domains automatically, change routes, bypass the proxy, or disable TLS,
   apt signatures, or Go checksums. If a republished policy is not active,
   diagnose a platform version mismatch before retrying builds.
3. Verify tools: Linux amd64 (the currently validated architecture), Python
   **3.11+**, Bash, curl, Git, Docker/Compose, `flock`, and host Cargo/Rust for the
   wasm-bindgen helper source build. Browser checks additionally need Chromium
   and Python Playwright. Availability is instance-specific: inspect it rather
   than assuming it is absent or persists from another session.
4. Run `git status --short` and `df -h .`. Inspect retained images before
   rebuilding. Docker `vfs` on a 32 GB disk needs an **exclusive build window**;
   stop disposable stacks before cold builds and never overlap cold builds.
   The cloud build command serializes other invocations of itself with a
   per-user lock. It cannot lock unrelated Docker commands.
5. In a sandbox, Docker socket access, browser launch, or proxy connections may
   require the execution tool's normal `require_escalated` approval. A sandbox
   `operation not permitted` is different from a proxy `Domain forbidden`.
   Do not start another daemon or change its storage driver to fix either.

## Quick start

Run from the repository root. No `/workspace/Noms` path is hard-coded.

```bash
# Optional: use an explicit persistent workspace path OUTSIDE this checkout.
export NOMS_CLOUD_STATE_DIR=/workspace/noms-cloud-state/my-worktree
# Default is a checkout-specific directory under ${TMPDIR:-/tmp}/noms-codex-cloud.
# Override only if the platform supplies a different combined public CA bundle.
export NOMS_CLOUD_CA=/etc/ssl/certs/ca-certificates.crt

bash docker/codex-cloud/cloud.sh doctor
bash docker/codex-cloud/cloud.sh prepare
bash docker/codex-cloud/cloud.sh build cloud
bash docker/codex-cloud/cloud.sh up cloud
```

`doctor` checks the policy snapshot, managed Docker socket, actual HTTP PGDG
InRelease access, and free space. It does not replace `environment_status`.
`prepare` refreshes Docker's default proxy IP from the inherited proxy, preserves
registry credentials/settings and other daemon proxy entries, and generates
cloud adaptations in the external state directory. It never prints credentials.

`build` builds PostgreSQL, development, and the smaller tools image sequentially.
Build logs are in `$NOMS_CLOUD_STATE_DIR/logs`. On `vfs`, after **each** image it
**prunes the default builder's entire unused build cache**, flattens the image,
checks complete runtime configuration/platform preservation, retags the compact
result, and removes only the superseded image ID (without force). This trades
compilation cache for disk space. Use this command only in the exclusive build
window above; do not use it on a shared builder whose caches must be retained.
Docker registry/base-image caches can still consume space; inspect them rather
than using a blanket image/volume prune.

The build command also downloads/builds the pinned Dioxus helper tools. To
restore only those tools, run `bash docker/codex-cloud/cloud.sh tools`.
Both source builds and downloads keep TLS/checksum verification enabled.

`up` requires all three local images and helpers; it never starts automatic
Compose builds. It preserves the shared lifecycle's ownership/token validation,
worktree lock, per-instance volumes, dynamic loopback ports, schema initialization,
and startup slot. It removes its validated temporary bootstrap containers
before final creation so `vfs` does not briefly duplicate their roots. The app
gets a 20-minute health-check start period for cold compilation.

The command prints endpoints and **stays attached** to app logs. Keep that
execution session alive while testing. Another terminal/session can run:

```bash
bash docker/codex-cloud/cloud.sh status cloud
bash docker/codex-cloud/cloud.sh browser cloud
# Destructive only to this disposable instance: removes its DB/storage/build volumes.
bash docker/codex-cloud/cloud.sh down cloud
```

The instance name `cloud` becomes `noms-cloud`. An active duplicate is rejected.
Use a separate Git worktree, instance name, and external state directory for a
second agent. Runtime metadata defaults to `/tmp/noms-dev/noms-cloud/metadata`.
Do not print the sibling `instance.env` or saved browser cookies/session tokens.
Reusing a state directory after dependency/CLI upgrades requires reviewing pins
and regenerating/rebuilding; do not blindly reuse images from an unrelated HEAD.

## Reuse existing verified images

If this instance retained completed images, inspect their provenance and the
source Dockerfile/lockfile versions first. After confirming compatibility, tag
those existing images for the selected instance instead of doing a cold build:

```bash
# Replace these SOURCE placeholders with actual verified local image references.
docker tag SOURCE_DEVELOPMENT noms-development:noms-cloud
docker tag SOURCE_POSTGRES noms-codex-postgres:noms-cloud
docker tag SOURCE_TOOLS noms-codex-tools:noms-cloud
bash docker/codex-cloud/cloud.sh prepare
bash docker/codex-cloud/cloud.sh tools
bash docker/codex-cloud/cloud.sh up cloud
```

Images use local tags because they are built from the repository's digest-pinned
bases and pinned tools. This workflow does not push images or publish an app.
Files on the current instance, Docker layers, volumes, and late-written workspace
snapshots are **not guaranteed permanent**. This committed folder is the durable
recipe. Inspect what survived in each replacement instance and rebuild only
missing artifacts; never assume a previous handoff implies a live process.

## Browser verification and sending screenshots

The validated executor had `/usr/bin/chromium` and Python Playwright 1.62.0.
Check the actual instance:

```bash
command -v chromium chromium-browser
python3 -c 'import playwright; print("Playwright available")'
chromium --version
```

If missing, provision them through the managed environment's supported setup.
For Python tooling, an external virtual environment with `pip install
playwright==1.62.0` is an option when permitted; use its Python to run the script.
A browser download may need additional reviewed destination permissions and
system libraries. Do not bypass a denial or assume `playwright install` is
permitted. Prefer an already installed Chromium; no browser download is needed
in the verified environment.

```bash
# Keep artifacts outside the source tree; choose the platform's output directory
# instead when available and writable so screenshots can be attached to chat.
export NOMS_BROWSER_OUTPUT=/workspace/noms-browser-artifacts/cloud
bash docker/codex-cloud/cloud.sh browser cloud
# Or invoke directly with an explicit executable/output path:
python3 docker/codex-cloud/browser-smoke.py --instance noms-cloud \
  --chromium /usr/bin/chromium --output "$NOMS_BROWSER_OUTPUT"
```

The script resolves the actual loopback URL from instance metadata, launches
headless Chromium, loads home/WASM, clicks the theme button and checks DOM plus
localStorage (hydration), and clicks Sign In. It writes `home.png`,
`home-dark.png`, `login.png`, and `result.json`, closes the browser, and fails on
page errors or failed WASM delivery. External browser traffic uses the inherited
proxy; only loopback is excluded. TLS errors are not ignored. Browser sandbox
restrictions may require the execution tool approval mentioned above; this is
not a network-policy bypass.

Agents should inspect relevant screenshots and attach/embed them using the
client's supported artifact mechanism. A screenshot sitting on the executor is
not automatically visible to the user. Share screenshots for visual changes,
failures, or requested evidence. Review authenticated screenshots for personal
information before sharing. This is a smoke check, not a complete browser suite:
extend tests for the changed flow (mock OAuth, uploads, recipe editing, keyboard
navigation, mobile viewport, etc.) and report exactly what was exercised.

## Why the cloud adaptations exist

| Adaptation | Reason |
| --- | --- |
| CA secret on every Dockerfile RUN | Inner containers do not inherit the executor's interception CA. Secret mode 0444 lets apt's unprivileged downloader read the public bundle. |
| Tool-specific CA settings and apt CaInfo | TLS clients use different trust settings; verification stays enabled. |
| Explicit `chmod 755` on copied PostgreSQL init script | Restrictive checkout modes plus `chmod +x` can leave it unreadable to `postgres`. |
| Smaller tools image | MinIO and schema initialization do not need the Rust toolchain; smaller containers reduce `vfs` duplication. |
| Image flattening/cache cleanup | Full development layers and concurrent recreation can exhaust a 32 GB filesystem. |
| Local wget health override | BusyBox wget sent the loopback health request through the external proxy despite no_proxy. Only this local check uses `-Y off`. |
| Read-only runtime tool cache mount | Dioxus CLI 0.7.9's helper downloader uses built-in webpki roots and ignores the cloud CA environment settings. |
| External generated Tailwind CSS mount | CSS generation must not modify the tracked `assets/tailwind.css`. |

Dioxus helpers are pinned to esbuild 0.27.3, wasm-bindgen 0.2.121, and Tailwind
4.1.5. esbuild's npm SHA-512 integrity and a pinned binary SHA-256 are checked;
Tailwind's canonical release binary has a pinned SHA-256; wasm-bindgen is built
from its exact crate version with `cargo install --locked` and Go/Cargo checksum
verification is never disabled. Binary checksums are recorded in external
`runtime-tools.sha256`; no binaries, CA files, proxy addresses, credentials,
logs, screenshots, generated Dockerfiles, or runtime state belong in Git.
The generated adaptations fail if the upstream lifecycle insertion points change;
review the integration rather than weakening its ownership checks.

## Verification and troubleshooting

```bash
python3 -m unittest discover -s docker/codex-cloud -p 'test_*.py' -v
bash -n docker/codex-cloud/cloud.sh docker/codex-cloud/app-entrypoint.sh
```

Configuration regressions run in CI; browser checks need a live local stack.
Do not claim old server/configuration test runs were repeated. After startup,
check init exit codes and health, actual browser interactions, and the database/
storage behavior relevant to the task. Keep the validation scope in the handoff.

- **HTTP 403 Domain forbidden:** compare the active version, policy snapshot,
  status enforcement, and exact request. Dockerfile edits cannot fix a denial.
- **UnknownIssuer during Dioxus helper download:** restore the pinned cache with
  `cloud.sh tools`; do not disable verification or bake a session CA into layers.
- **Docker proxy points to an old session IP:** rerun `cloud.sh prepare`; recreate
  old containers through their owning lifecycle helper, since their env is fixed.
- **Disk full:** stop this instance, inspect `df`, images and build caches, and
  reuse/compact verified images. Never overlap cold builds, blanket-prune another
  agent's volumes, or change the storage driver. The compactor rejects nonempty
  declared image volumes and never snapshots a live database container.
- **Failed instance name remains:** inspect its preserved runtime diagnostics,
  then run `cloud.sh down NAME` before retrying. Remove only its owned resources.
- **No user-accessible URL:** expected. Local browser automation works inside the
  executor; a public hosting deployment or platform-supported preview is a
  separate task with app/OAuth/media URL configuration.
