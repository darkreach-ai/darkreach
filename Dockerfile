# Multi-stage Docker build for darkreach
#
# Stage 1: Build the Rust release binary (with GMP)
# Stage 2: Minimal runtime image with binary + migrations
#
# The image is role-agnostic — CMD defaults to coordinator (dashboard),
# but can run any darkreach subcommand (work, verify, etc.).
# Health checks and port mappings are handled per-service in compose.
#
# Usage:
#   docker build -t darkreach .
#   docker run -e DATABASE_URL=postgres://... -p 7001:7001 darkreach
#
# Build args:
#   RUST_TARGET_CPU  - CPU target for RUSTFLAGS (default: native for multi-arch)
#
# Worker mode:
#   docker run -e DATABASE_URL=postgres://... darkreach work --search-job-id 1

# ── Stage 1: Rust build ─────────────────────────────────────────
FROM rust:1-bookworm AS rust-build
WORKDIR /app

# Default to native so multi-arch (amd64/arm64) builds optimize per platform
ARG RUST_TARGET_CPU=native
ENV RUSTFLAGS="-C target-cpu=${RUST_TARGET_CPU}"

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgmp-dev m4 pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies by building with a dummy main first
COPY Cargo.toml Cargo.lock ./
COPY gwnum-sys/ gwnum-sys/
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && echo 'pub fn dummy() {}' > src/lib.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

COPY src/ src/
# Create stub bench/test files so Cargo.toml parses (originals in .dockerignore)
RUN mkdir -p benches tests \
    && for b in sieve_bench kbn_bench proof_bench core_bench flint_bench; do \
         echo "fn main(){}" > "benches/${b}.rs"; \
       done
# Touch main.rs so cargo rebuilds it (not the cached dummy)
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --lib --bins

# ── Stage 2: Runtime ─────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# OCI metadata labels
LABEL org.opencontainers.image.title="darkreach" \
      org.opencontainers.image.description="Distributed platform for hunting special-form prime numbers" \
      org.opencontainers.image.source="https://github.com/darkreach-ai/darkreach" \
      org.opencontainers.image.licenses="MIT"

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgmp10 ca-certificates curl postgresql-client \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-build /app/target/release/darkreach /usr/local/bin/darkreach

# Include migrations for self-contained DB init
COPY supabase/migrations/ /app/migrations/
COPY scripts/init-db-docker.sh /app/init-db-docker.sh
RUN chmod +x /app/init-db-docker.sh

ENTRYPOINT ["darkreach"]
CMD ["dashboard", "--port", "7001"]
