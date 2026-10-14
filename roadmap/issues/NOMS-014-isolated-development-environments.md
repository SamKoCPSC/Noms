# NOMS-014: Fully Isolated `just up` Development Environments

**Status:** ✅ Done
**Phase:** Developer infrastructure  
**Created:** 2026-07-24  
**Completed:** 2026-07-26
**Depends on:** NOMS-002 local development tooling

## Overview

Make every `just up` invocation a disposable development environment containing the application and all supporting infrastructure. Separate agent sessions must be able to run concurrently without sharing mutable runtime state, fixed ports, container names, data volumes, build output, browser session state, or teardown targets.

Each agent must work from a separate Git worktree to isolate source changes. The environments may share host-level immutable resources such as Docker image layers, but one invocation must never read, modify, stop, or delete another invocation's application state.

## Isolation contract

Every invocation owns a unique instance ID and uses it to scope:

- The Docker Compose project, container-generated names, network, and named volumes.
- PostgreSQL and MinIO data.
- The application container and its Dioxus session cache.
- Cargo build output and any writable dependency caches used by the container.
- Published host ports and browser-facing URLs.
- Generated runtime environment, state metadata, and logs.
- Session and OAuth cookie names.
- Health checks, status commands, and teardown.

The shared Docker daemon, host CPU, host memory, Docker image layers, and other immutable download caches are outside this isolation boundary. A source checkout is mutable state: two concurrent invocations from the same worktree must fail fast with an actionable message rather than pretend to be isolated.

## User experience

The default command remains:

```bash
just up
```

It generates a unique instance ID, starts a fresh environment, and prints its endpoints:

```text
Noms instance: noms-7f3a91
Application:   http://localhost:41783
MinIO console: http://localhost:41784
Runtime state: /tmp/noms-dev/noms-7f3a91
```

An optional explicit name supports stable, recognizable environments:

```bash
just up instance=agent-a
```

Explicit names must be normalized and validated as safe Compose project identifiers. An active duplicate name must be rejected without affecting the existing environment.

## Acceptance criteria

### AC1: Per-invocation Compose isolation

- [x] `just up` generates a collision-resistant instance ID unless an explicit instance name is supplied.
- [x] Every Compose command uses the same explicit project name.
- [x] The Compose file does not define fixed `container_name` values.
- [x] Application, PostgreSQL, MinIO, MinIO initialization, and Mock OAuth run inside the instance's Compose project.
- [x] Networks and writable volumes are project-scoped and unique to the invocation.
- [x] Starting or stopping one instance does not restart, stop, inspect, or remove another instance's resources.

### AC2: Dynamic ports and runtime configuration

- [x] No browser-facing or infrastructure service requires a fixed host port.
- [x] Published ports bind to loopback and are allocated without conflicting with concurrent instances.
- [x] The startup workflow discovers the assigned ports and generates a private per-instance runtime environment.
- [x] The app receives the correct `BASE_URL`, database URL, MinIO endpoint/public URL, and OAuth authorization/token/userinfo URLs.
- [x] Server-to-server traffic uses Compose DNS and container ports where browser access is unnecessary.
- [x] MinIO CORS permits only the current instance's application origin.
- [x] Startup prints the instance ID and usable browser-facing endpoints.

### AC3: Application and build isolation

- [x] The Dioxus development server runs in the instance's application container with hot reload.
- [x] The container bind-mounts only the invoking agent's worktree as source.
- [x] Cargo target output, Dioxus session data, and other writable build/runtime caches cannot overlap another instance.
- [x] Two active instances do not contend for `.sqlx` generation; `just up` does not rewrite shared generated source artifacts.
- [x] Concurrent use of the same worktree is detected and rejected before containers are started.
- [x] Separate worktrees can run and rebuild concurrently.

### AC4: Browser and authentication isolation

- [x] Each instance uses a distinct application origin or instance-specific session and OAuth cookie names.
- [x] Signing into one instance cannot authenticate, sign out, or invalidate the browser session of another instance.
- [x] OAuth redirects return to the correct dynamically assigned application origin.
- [x] Mock OAuth browser endpoints remain reachable through the printed instance URL configuration.
- [x] MinIO-hosted media URLs resolve through the current instance rather than another running stack.

### AC5: Lifecycle and recovery

- [x] Normal exit and `Ctrl+C` tear down only the current Compose project.
- [x] Normal teardown removes the instance's containers, network, and writable data volumes.
- [x] Cleanup is idempotent and safe after partial startup.
- [x] A failed health check or migration reports the instance ID, preserves useful logs, and exits non-zero.
- [x] Interrupted or abandoned environments can be listed and explicitly removed by instance ID.
- [x] Cleanup commands validate their targets and cannot expand to unrelated Compose projects.

### AC6: Concurrency verification

- [x] An automated integration test starts at least two isolated instances concurrently from separate worktrees.
- [x] The test proves distinct project names, containers, networks, volumes, host ports, runtime directories, and cookie names.
- [x] Database writes and MinIO objects created in one instance are absent from the other.
- [x] Both applications complete an independent Mock OAuth sign-in flow.
- [x] Stopping the first instance leaves the second healthy and usable.
- [x] Teardown leaves no per-test containers, networks, volumes, locks, or runtime directories.

### AC7: Documentation and compatibility

- [x] `README.md` documents the isolated `just up` workflow, printed endpoints, worktree requirement, status/cleanup commands, and troubleshooting.
- [x] `just down` has unambiguous instance semantics and cannot accidentally target every active environment.
- [x] Existing standalone migration, test, lint, and CI recipes continue to work.
- [x] The design works on the repository's supported Linux and macOS Docker environments.

## Verification evidence

- `bash scripts/tests/test-isolated-dev-config.sh`: 37 assertions passed, including
  exact cleanup ownership, Docker 29 label formatting, and a behavioral proof
  that different `NOMS_RUNTIME_ROOT` values cannot bypass the same-worktree lock.
- `bash scripts/tests/test-isolated-dev-integration.sh`: two fresh worktrees
  completed concurrent startup with distinct projects, ports, runtime
  directories, build/data volumes, database and MinIO state, and session cookie
  names. Both completed the form-driven Mock OAuth flow, accepted only their own
  sessions, and remained independent through logout and targeted teardown. No
  test containers, networks, volumes, locks, runtime directories, or worktrees
  remained afterward.
- `just check`, `cargo fmt --check`, Bash syntax checks, the focused
  per-instance-cookie unit test, and `git diff --check` passed.
- The independent implementation review passed with no blocking or should-fix
  findings after its Docker 29 and lock-scope findings were corrected.
- Repository-wide baseline note: `just test` still has two unrelated,
  pre-existing upload tests that contend on global storage initialization, and
  `just ci` still stops on 28 pre-existing Clippy warnings in unchanged recipe
  image-editor files. NOMS-014 did not change those files or recipes.
- Linux/Rancher Desktop received the full live verification. The orchestration
  uses Docker Compose v2 and Bash facilities available on the documented macOS
  Docker Desktop path, includes GNU/BSD hashing fallbacks, and avoids
  daemon-side bind mounts of the host runtime directory.

## Out of scope

- Running each agent in a separate virtual machine or separate Docker daemon.
- Preventing normal contention for host CPU, memory, disk bandwidth, or network bandwidth.
- Disabling Docker image-layer reuse.
- Isolating two agents that intentionally edit the same Git worktree.
- Changing staging or production deployment topology.

## Outcome

Agents can run `just up` concurrently from separate worktrees and receive independent, disposable Noms environments. Each environment includes the hot-reloading application, database, object storage, and authentication mock, with no shared mutable runtime state and no possibility of one instance's normal cleanup taking down another.
