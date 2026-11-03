# Database test isolation

Application tests create independent PostgreSQL clusters with pgtemp 0.7.1.
Its default connection URI uses localhost and a randomly selected TCP port.
The port is released before initdb and PostgreSQL startup, so a concurrent
start can select the same port. The default user, password and database name
are identical across clusters. A client can therefore connect successfully
to another test's server rather than fail authentication.

This explains otherwise unrelated failures while applying fixture DDL:
concurrent table creation can violate pg_type_typname_nsp_index even with
CREATE TABLE IF NOT EXISTS, and shutting down one cluster can reset another
test's connections or leave its pool waiting for an available connection.

A recent example is [the first attempt of run 41683313112](https://github.com/SamKoCPSC/Noms/actions/runs/41683313112/attempts/1):
two router tests failed in users-table/index setup with a catalog uniqueness
error and a pool timeout. The unchanged run passed on retry. Earlier vocabulary
and collection tests showed the same catalog conflicts and connection resets.

## Private endpoints

The test-only TestDatabase helper disables TCP listening and exports the
Unix socket inside the cluster's unique data directory. The port number is
still part of the socket filename, but equal ports in different directories
cannot address the same server. The owning handle retains pgtemp's process
shutdown and directory cleanup.

Both the application fixture helper and vocabulary tests use this endpoint.
The exported URI supports SQLx, psql and local browser fixtures. Socket paths
are percent encoded, including spaces; URI query values must not use the
form-encoding '+' convention for spaces.

No production database settings, application queries, schema or test
parallelism change. These tests remain concurrent and do not retry failed
application assertions or silently ignore database setup errors.

## Verification

The regression tests force two simultaneous clusters onto the same port.
They verify each connection's data directory and Unix transport, create the
same table independently, store different rows, then stop one cluster and
continue querying the other. Another test checks SQLx and psql against a
socket directory containing spaces.

Run the focused regressions with:

```sh
cargo test --locked --no-default-features --features server test_database:: -- --test-threads=8
```

The full CI suite also covers runtime-grant scripts that consume the exported
URI. Temporary reproduction and repeated-suite workflows are kept outside the
application PR.
