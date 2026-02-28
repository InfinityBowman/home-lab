# Stage 1: Build the Rust binary
FROM rust:1.91-bookworm AS builder
WORKDIR /app

# Copy workspace manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Copy migrations (embedded by sqlx at compile time)
COPY migrations/ migrations/

RUN cargo build --release --bin homelab-api

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates git curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r homelab && useradd -r -g homelab -u 1001 homelab \
    && mkdir -p /data /git-repos \
    && chown homelab:homelab /data /git-repos

COPY --from=builder /app/target/release/homelab-api /usr/local/bin/homelab-api
COPY migrations/ /app/migrations/

ENV DATABASE_URL=sqlite:///data/homelab.db?mode=rwc
EXPOSE 5170

USER homelab
CMD ["homelab-api"]
