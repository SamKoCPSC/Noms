# NOMS-014: Fully Isolated `just up` Development Environments

**Status:** 🔵 Ready  
**Phase:** Developer infrastructure  
**Created:** 2026-07-24  
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

- [ ] `just up` generates a collision-resistant instance ID unless an explicit instance name is supplied.
- [ ] Every Compose command uses the same explicit project name.
- [ ] The Compose file does not define fixed `container_name` values.
- [ ] Application, PostgreSQL, MinIO, MinIO initialization, and Mock OAuth run inside the instance's Compose project.
- [ ] Networks and writable volumes are project-scoped and unique to the invocation.
- [ ] Starting or stopping one instance does not restart, stop, inspect, or remove another instance's resources.

### AC2: Dynamic ports and runtime configuration

- [ ] No browser-facing or infrastructure service requires a fixed host port.
- [ ] Published ports bind to loopback and are allocated without conflicting with concurrent instances.
- [ ] The startup workflow discovers the assigned ports and generates a private per-instance runtime environment.
- [ ] The app receives the correct `BASE_URL`, database URL, MinIO endpoint/public URL, and OAuth authorization/token/userinfo URLs.
- [ ] Server-to-server traffic uses Compose DNS and container ports where browser access is unnecessary.
- [ ] MinIO CORS permits only the current instance's application origin.
- [ ] Startup prints the instance ID and usable browser-facing endpoints.

### AC3: Application and build isolation

- [ ] The Dioxus development server runs in the instance's application container with hot reload.
- [ ] The container bind-mounts only the invoking agent's worktree as source.
- [ ] Cargo target output, Dioxus session data, and other writable build/runtime caches cannot overlap another instance.
- [ ] Two active instances do not contend for `.sqlx` generation; `just up` does not rewrite shared generated source artifacts.
- [ ] Concurrent use of the same worktree is detected and rejected before containers are started.
- [ ] Separate worktrees can run and rebuild concurrently.

### AC4: Browser and authentication isolation

- [ ] Each instance uses a distinct application origin or instance-specific session and OAuth cookie names.
- [ ] Signing into one instance cannot authenticate, sign out, or invalidate the browser session of another instance.
- [ ] OAuth redirects return to the correct dynamically assigned application origin.
- [ ] Mock OAuth browser endpoints remain reachable through the printed instance URL configuration.
- [ ] MinIO-hosted media URLs resolve through the current instance rather than another running stack.

### AC5: Lifecycle and recovery

- [ ] Normal exit and `Ctrl+C` tear down only the current Compose project.
- [ ] Normal teardown removes the instance's containers, network, and writable data volumes.
- [ ] Cleanup is idempotent and safe after partial startup.
- [ ] A failed health check or migration reports the instance ID, preserves useful logs, and exits non-zero.
- [ ] Interrupted or abandoned environments can be listed and explicitly removed by instance ID.
- [ ] Cleanup commands validate their targets and cannot expand to unrelated Compose projects.

### AC6: Concurrency verification

- [ ] An automated integration test starts at least two isolated instances concurrently from separate worktrees.
- [ ] The test proves distinct project names, containers, networks, volumes, host ports, runtime directories, and cookie names.
- [ ] Database writes and MinIO objects created in one instance are absent from the other.
- [ ] Both applications complete an independent Mock OAuth sign-in flow.
- [ ] Stopping the first instance leaves the second healthy and usable.
- [ ] Teardown leaves no per-test containers, networks, volumes, locks, or runtime directories.

### AC7: Documentation and compatibility

- [ ] `README.md` documents the isolated `just up` workflow, printed endpoints, worktree requirement, status/cleanup commands, and troubleshooting.
- [ ] `just down` has unambiguous instance semantics and cannot accidentally target every active environment.
- [ ] Existing standalone migration, test, lint, and CI recipes continue to work.
- [ ] The design works on the repository's supported Linux and macOS Docker environments.

## Out of scope

- Running each agent in a separate virtual machine or separate Docker daemon.
- Preventing normal contention for host CPU, memory, disk bandwidth, or network bandwidth.
- Disabling Docker image-layer reuse.
- Isolating two agents that intentionally edit the same Git worktree.
- Changing staging or production deployment topology.

## Outcome

Agents can run `just up` concurrently from separate worktrees and receive independent, disposable Noms environments. Each environment includes the hot-reloading application, database, object storage, and authentication mock, with no shared mutable runtime state and no possibility of one instance's normal cleanup taking down another.
