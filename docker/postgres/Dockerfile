FROM postgres:17-bookworm

# Install pg_cron from the PostgreSQL apt repository (already configured in the base image)
RUN apt-get update \
    && apt-get install -y --no-install-recommends postgresql-17-cron \
    && rm -rf /var/lib/apt/lists/*
