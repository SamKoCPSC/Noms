# NOMS-014: Fully Isolated `just up` Development Environments — Implementation Plan

**Issue:** [NOMS-014-isolated-development-environments.md](../issues/NOMS-014-isolated-development-environments.md)  
**Created:** 2026-07-24  
**Depends on:** NOMS-002 local development tooling  
**Approach:** Establish a single per-invocation identity, move the development app into the Compose stack, allocate browser-facing ports dynamically, generate runtime configuration after container creation, isolate every writable path, and prove the contract with a two-instance integration test.

---

## Implementation contract

### Meaning of isolation

This work guarantees isolation of mutable application and development state:

- source checkout;
- containers and Compose lifecycle;
- networks and service discovery;
- database and object-storage contents;
- host ports and public URLs;
- application build output and runtime caches;
- browser cookies and OAuth state;
- generated runtime configuration and logs.

Docker's daemon, image layers, host resources, and explicitly read-only caches may remain shared. Separate Git worktrees are mandatory because a shared source checkout is itself shared mutable state.

### Instance identity

Create one normalized instance ID before any resource is created. Automatic IDs should combine a `noms-` prefix with collision-resistant random data. An optional `instance=` argument provides a stable suffix for humans and automation.

The instance ID must:

- satisfy Docker Compose project-name constraints;
- be passed explicitly to every Compose command;
- appear in the runtime directory and worktree lock metadata;
- be printed in startup, error, status, and cleanup output;
- never be reconstructed from an unreliable implicit Compose default.

### Runtime directory

Each invocation owns a private directory below a configurable runtime root, defaulting to:

```text
/tmp/noms-dev/<instance-id>/
├── instance.env
├── metadata
├── compose.log
└── worktree.lock
```

Create files with permissions that do not expose session secrets. Metadata records the canonical worktree, Compose project, process identity, creation time, and discovered endpoints. It must contain enough information for targeted status and recovery commands without becoming an authority for deleting arbitrary resources.

### Port assignment

Compose publishes required ports on `127.0.0.1` with Docker-assigned host ports. Use a two-stage lifecycle:

1. Create the project containers without starting application processes.
2. Query the assigned host ports.
3. Write the final per-instance runtime environment.
4. Start the stack with dependency health conditions.

Entrypoints that need discovered values source the mounted runtime environment when the container starts. If the installed Compose implementation cannot report bindings for created containers, use a bounded allocate-and-retry strategy that treats a bind conflict as a startup retry, never as permission to attach to an existing container.

### Endpoint split

Use internal Compose DNS for server-to-server requests:

```text
postgres:5432
minio:9000
mock-oauth:8080
```

Use discovered loopback ports for browser-facing URLs:

- application origin and OAuth callback;
- Mock OAuth authorization pages;
- MinIO public media;
- MinIO console when requested.

Keep OAuth authorization URLs browser-facing while token and userinfo URLs use the internal network where supported by the provider mock.

---

## Checkpoint 1: Instance orchestrator and safety boundary

**Files:** `justfile`, new development orchestration script, `.gitignore`, `README.md`

### Tasks

- Make `just up` delegate to one long-lived orchestration process so its instance ID and cleanup trap cannot vary between recipe lines.
- Generate or validate the instance ID before invoking Compose.
- Create the private runtime directory and metadata.
- Acquire a non-blocking lock keyed by the canonical worktree path.
- Reject a second concurrent invocation from the same worktree with guidance to create a separate worktree.
- Centralize Compose invocation in one helper that always supplies the explicit project name and Compose file.
- Install an idempotent cleanup trap only after the project identity has been validated.
- Add targeted `status`, `logs`, and recovery cleanup operations keyed by exact instance ID.

### Verify

- Invalid explicit instance names fail before Docker resources are created.
- Two orchestrators cannot claim the same instance or worktree.
- Every emitted Compose command contains the expected project name.
- A simulated early failure invokes cleanup only for the current project.
- `git diff --check` passes.

**Risk:** High. A project-name or cleanup bug can cross the isolation boundary.

---

## Checkpoint 2: Compose project isolation and fresh state

**Files:** `docker-compose.yml`, development container definitions, orchestration script

### Tasks

- Remove every fixed `container_name`.
- Add the Dioxus development app as a Compose service.
- Keep PostgreSQL, MinIO, MinIO initialization, and Mock OAuth in the same project.
- Ensure all networks and writable volumes use Compose-generated project scoping.
- Add project-scoped volumes for PostgreSQL data, MinIO data, Cargo target output, and Dioxus session/runtime data.
- Define health checks and dependency conditions so initialization and schema application complete before the app reports ready.
- Move database extension/schema application into an idempotent project-scoped initialization job or app startup phase.
- Do not run `cargo sqlx prepare` as a side effect of `just up`.
- Bind-mount the invoking worktree for hot reload without mounting another worktree or shared build directory.

### Verify

- `docker compose config` contains no fixed container, network, or volume names.
- Two project names produce disjoint resource names.
- Fresh instances begin with empty PostgreSQL and MinIO volumes.
- Application source changes in one worktree trigger only its own Dioxus rebuild.
- The app becomes healthy only after infrastructure and schema initialization succeed.

**Risk:** High. Development-container ownership, file permissions, and hot reload must work consistently across supported hosts.

---

## Checkpoint 3: Dynamic ports and generated environment

**Files:** `docker-compose.yml`, orchestration script, container entrypoints, `.env.local` documentation

### Tasks

- Replace fixed host mappings for the app, PostgreSQL, MinIO, MinIO console, and Mock OAuth with loopback-only dynamic publishing.
- Create containers, discover their assigned bindings, and validate that every required endpoint is unique and reachable.
- Generate `instance.env` with the discovered browser URLs and internal service URLs.
- Mount the runtime environment read-only into services that need it and source it at process startup.
- Configure:
  - internal `DATABASE_URL` and `PG*` values;
  - internal `R2_ENDPOINT`;
  - browser-facing `R2_PUBLIC_URL`;
  - browser-facing OAuth authorization URLs;
  - internal OAuth token and userinfo URLs;
  - dynamic `BASE_URL`;
  - dynamic MinIO CORS origin.
- Keep committed or developer-owned `.env.local` unchanged during startup.
- Print endpoints only after health verification succeeds.

### Verify

- No required service binds fixed ports `5432`, `8080`, `8082`, `9000`, or `9001` on the host.
- All published listeners bind to loopback.
- Two concurrently created projects receive distinct host bindings.
- The app connects to its own PostgreSQL and MinIO services through Compose DNS.
- Browser-visible media and authorization URLs use the correct discovered ports.
- A forced allocation conflict is retried or fails cleanly without attaching to another instance.

**Risk:** High. Internal and browser-facing endpoint confusion can break OAuth callbacks or media delivery while basic health checks still pass.

---

## Checkpoint 4: Application, cache, and browser-session isolation

**Files:** development app container, `src/main.rs`, authentication/session configuration, Dioxus startup configuration

### Tasks

- Start Dioxus on a fixed container port and publish it through the instance's dynamic host binding.
- Give Dioxus a project-scoped `--session-cache-dir`.
- Set Cargo target and other writable tool directories to project-scoped paths.
- Avoid writable global cache mounts; use project volumes or read-only shared inputs followed by instance-local writes.
- Add configuration for an instance-specific session cookie name and any OAuth state/correlation cookies.
- Ensure the dynamic external origin is used for OAuth callback construction.
- Confirm local storage, service workers, websocket hot reload, and cookies are scoped to the current application origin.
- Keep production cookie names and deployment behavior unchanged when the instance configuration is absent.

### Verify

- Concurrent builds write to different target and Dioxus cache paths.
- Signing into instance A does not authenticate instance B.
- Signing out of instance A does not affect instance B.
- OAuth callbacks return to the originating instance.
- Hot reload websocket traffic connects to the correct application origin.
- Production/default session configuration remains backward compatible.

**Risk:** High. Cookies ignore ports for domain matching, so port isolation alone is insufficient.

---

## Checkpoint 5: Exact lifecycle, diagnostics, and recovery

**Files:** `justfile`, orchestration script, `README.md`

### Tasks

- Make `Ctrl+C`, normal exit, and startup failure target only the current explicit Compose project.
- Remove project containers, networks, and writable volumes during normal disposable teardown.
- Preserve or stream useful service logs before cleanup when startup fails.
- Make repeated cleanup harmless.
- Add a command to list Noms development instances from validated metadata and Compose labels.
- Add targeted `just down instance=<id>` behavior; do not make an unqualified command delete every project.
- Detect stale metadata and locks by comparing process state and Compose labels.
- Require an explicit, validated instance ID for abandoned-environment cleanup.
- Never use broad name globs as deletion targets.

### Verify

- Stopping instance A leaves instance B healthy.
- Cleanup after partial container creation succeeds.
- Repeating cleanup does not fail or broaden its target.
- A forged metadata file cannot redirect cleanup to a non-Noms Compose project.
- Failure output includes instance ID, service state, and the relevant log location.

**Risk:** High. Recovery paths must maintain the same isolation guarantees as the happy path.

---

## Checkpoint 6: Two-instance integration test

**Files:** new integration test script under `scripts/tests/`, `justfile`

### Tasks

- Create two temporary Git worktrees from the same revision.
- Start `just up` concurrently in both worktrees with deterministic test instance names.
- Capture metadata and wait for both applications to become healthy.
- Assert distinct:
  - Compose project names;
  - container IDs;
  - networks;
  - PostgreSQL, MinIO, build, and runtime volumes;
  - published ports;
  - runtime directories;
  - session cookie names.
- Insert a unique database row and MinIO object into each instance and prove it is absent from the peer instance.
- Complete an independent Mock OAuth login in each application and prove cookies/sessions are not interchangeable.
- Stop one instance and exercise the remaining application's health, database, storage, and authenticated route.
- Tear down both stacks and verify no test resources or locks remain.
- Register the test as a focused `just` recipe; keep it out of routine unit-only commands if its Docker cost is too high for every local run.

### Verify

- Run the test repeatedly and with startup order reversed.
- Run a forced-failure case during initialization.
- Inspect Docker resources after completion for leaks.
- Run the existing CI gate to verify standalone development and production behavior remain intact.

**Risk:** Medium. The test must be strict enough to catch cross-project access without becoming timing-sensitive.

---

## Checkpoint 7: Documentation and final review

**Files:** `README.md`, `.env.example` or local-environment documentation, roadmap issue

### Tasks

- Document the worktree prerequisite and provide example worktree commands.
- Document automatic and explicit instance names.
- Explain printed application, OAuth, MinIO, log, and runtime paths.
- Document targeted status, logs, teardown, and stale-instance recovery.
- Explain which host-level resources remain shared and why they do not violate the mutable-state isolation contract.
- Add troubleshooting for port allocation, Docker health, file ownership, hot reload, and stale locks.
- Run two fresh instances and manually verify startup, editing/hot reload, database writes, image upload/read, Mock OAuth sign-in/sign-out, teardown, and continued operation of the peer instance.
- Update every acceptance criterion with evidence before marking NOMS-014 complete.

### Verify

- A developer unfamiliar with the implementation can start two worktrees and environments using only the README.
- Commands copied from the documentation work without hidden environment setup.
- The complete integration test passes.
- `just ci` passes.
- `git diff --check` passes.

**Risk:** Low. Documentation is part of the operational safety boundary because recovery commands are destructive when misused.

---

## Recommended implementation order

1. Build the instance identity, worktree lock, and exact Compose wrapper.
2. Remove global Compose names and move the app into the project.
3. Add dynamic port discovery and runtime environment generation.
4. Isolate build caches, cookies, OAuth state, and public URLs.
5. Implement targeted teardown and recovery.
6. Add and repeatedly run the concurrent two-instance test.
7. Finish operational documentation and complete the full review.

Do not weaken the isolation contract to preserve the old fixed localhost ports. The printed endpoints and instance metadata replace that implicit global convention.
