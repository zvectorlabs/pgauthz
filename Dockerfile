# pgauthz Docker image
#
# Builds the extension from source and installs it into the official
# postgres base image.  Parameterized on PG_MAJOR so one Dockerfile
# covers pg16, pg17, and pg18.
#
# Build locally:
#   docker build --build-arg PG_MAJOR=17 -t pgauthz:dev .
#
# Run (extension is pre-installed, just CREATE EXTENSION):
#   docker run --rm -e POSTGRES_PASSWORD=secret pgauthz:dev
#   psql ... -c "CREATE EXTENSION pgauthz;"

ARG PG_MAJOR=16
ARG RUST_TOOLCHAIN=1.93
ARG PGRX_VERSION=0.17.0

# ── Builder ──────────────────────────────────────────────────────────────────
FROM postgres:${PG_MAJOR} AS builder
ARG PG_MAJOR
ARG RUST_TOOLCHAIN
ARG PGRX_VERSION

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    libclang-dev \
    clang \
    pkg-config \
    libssl-dev \
    ca-certificates \
    postgresql-${PG_MAJOR} \
    postgresql-server-dev-${PG_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (minimal profile — no docs/clippy needed here)
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
    --default-toolchain ${RUST_TOOLCHAIN} \
    --profile minimal
ENV PATH="/root/.cargo/bin:$PATH"

# Install cargo-pgrx — this layer is cached as long as PGRX_VERSION doesn't change
RUN cargo install cargo-pgrx --version ${PGRX_VERSION} --locked

WORKDIR /build
COPY . .

# Point pgrx at the system-installed pg_config (no download needed)
RUN cargo pgrx init --pg${PG_MAJOR} /usr/lib/postgresql/${PG_MAJOR}/bin/pg_config

# Build the release package — output lands in target/release/pgauthz-pg<N>-*/
RUN cargo pgrx package -p pgauthz \
    --pg-config /usr/lib/postgresql/${PG_MAJOR}/bin/pg_config \
    --features pg${PG_MAJOR} \
    --no-default-features \
    --profile release

# Collect the built files into /pgauthz-pkg/ preserving the OS path structure
# so we can COPY --from=builder /pgauthz-pkg/ / in the final stage.
RUN PKG_DIR=$(find /build/target/release -maxdepth 1 -type d -name "pgauthz-pg${PG_MAJOR}*" | head -1) \
    && mkdir -p /pgauthz-pkg \
    && cp -r "${PKG_DIR}/." /pgauthz-pkg/

# ── Final image ───────────────────────────────────────────────────────────────
FROM postgres:${PG_MAJOR}
ARG PG_MAJOR

# Copy .so + extension SQL/control files into the standard PG directories
COPY --from=builder /pgauthz-pkg/ /

# Sanity-check: verify the shared library is in the right place
RUN ls /usr/lib/postgresql/${PG_MAJOR}/lib/pgauthz.so \
    && ls /usr/share/postgresql/${PG_MAJOR}/extension/pgauthz.control

LABEL org.opencontainers.image.source="https://github.com/zvectorlabs/pgauthz" \
      org.opencontainers.image.description="Zanzibar-style authorization as a Postgres extension" \
      org.opencontainers.image.licenses="Apache-2.0"
