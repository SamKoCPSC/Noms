# Dependency security review — 2026-08-20

## Reproducible baseline

- Auditor: `cargo-audit 0.22.2`, installed with `--locked`.
- RustSec database revision: `2f08fbb85332687b721f2f22706d07448369451b`.
- Audited lockfile after CP6 updates: 775 packages.
- Production graph command: `cargo tree --no-default-features --features server`.

CI invokes `scripts/security-audit.sh`. Exceptions live only in
`security/rustsec-allowlist.txt`; the wrapper rejects malformed, duplicate, or
expired entries before invoking Cargo Audit.

## Vulnerabilities resolved

The lockfile was updated to patched releases for the current HTTP/2,
QUIC, browser-launch, crossbeam, error-handling, event-listener, and memory-map
advisories. The AWS S3/config dependencies were upgraded and their default
features narrowed so production no longer compiles the legacy Hyper 0.14 and
rustls 0.21 connector. The production HTTP graph now resolves `h2 0.4.16` and
`rustls-webpki 0.103.13`.

## Remaining vulnerability exception

`RUSTSEC-2023-0071` (`rsa 0.9.10`) has no patched release. The package remains
in Cargo.lock through SQLx's optional MySQL metadata, but the application sets
`default-features = false`, enables PostgreSQL only, and the production inverse
dependency trace reports no RSA path. The exception expires on 2026-11-30 and
must be removed or re-proven then.

## Informational warnings

- GTK3, `glib`, `paste`, `fxhash`, `proc-macro-error`, and `rand 0.7` warnings
  exist only in non-production desktop/target dependency metadata and have no
  server inverse-dependency path.
- `lru 0.16.4` is present through AWS S3 and Dioxus server. Its advisory
  requires a panicking `Drop` implementation for an LRU key plus caught unwind
  followed by reuse. These libraries use internal key types; Noms supplies no
  attacker-defined key or `Drop` implementation and does not catch an unwind
  around those caches. No reachable trigger was found. Re-evaluate when AWS or
  Dioxus permits `lru >= 0.18.2`.
- `enumset 1.1.12` and `spin 0.9.8` are yanked transitive releases in Dioxus and
  Axum/multer respectively. RustSec reports no vulnerability advisory for
  either package, and direct compatible upgrades are not available in the
  current upstream constraints.

None of the remaining items is a reachable unmitigated critical or high
advisory in the production server graph.
