FROM postgres:17-bookworm

# Install pg_cron from the PostgreSQL apt repository (already configured in the base image)
RUN apt-get update \
    && apt-get install -y --no-install-recommends postgresql-17-cron \
    && rm -rf /var/lib/apt/lists/*

# Bake shared_preload_libraries into the image's default PostgreSQL configuration.
# The official postgres Docker entrypoint copies postgresql.conf.sample to PGDATA
# on first init, so these settings become permanent defaults for any container
# started from this image — no -c command flags or volume-mounted configs needed.
#
# This guarantees identical behavior across:
#   - Local Docker Compose (no command: override needed)
#   - Railway (no way to pass command flags to managed containers)
#   - Any other container orchestration
#
# Using append ('>>') rather than sed replacement because:
#   1. The settings may not exist in the sample config as comments
#   2. PostgreSQL uses last-wins semantics, so appended values override earlier ones
#   3. Append is idempotent and doesn't break if the sample config format changes
RUN echo "shared_preload_libraries = 'pg_cron'" >> /usr/share/postgresql/postgresql.conf.sample

# Copy init script that configures cron.database_name to match POSTGRES_DB.
# pg_cron defaults to the "postgres" database, but we need it pointing at our
# application database so that CREATE EXTENSION pg_cron works in the right place.
# This runs once on first database init (docker-entrypoint.sh runs /docker-entrypoint-initdb.d/*).
COPY init-cron-config.sh /docker-entrypoint-initdb.d/init-cron-config.sh
RUN chmod +x /docker-entrypoint-initdb.d/init-cron-config.sh

# Health check — pg_isready returns 0 when the server is accepting connections.
HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=3 \
    CMD ["pg_isready", "-U", "${POSTGRES_USER:-noms}"]