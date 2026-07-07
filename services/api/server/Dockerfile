ARG RUST_VERSION=1.92.0

FROM rust:${RUST_VERSION}-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    clang \
    lld \
    ca-certificates \
    curl \
    make \
    autoconf \
    automake \
    libtool \
    zlib1g-dev \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

WORKDIR /app

# ============================================
# Planner
# ============================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ============================================
# Dependencies
# ============================================
FROM chef AS dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

# ============================================
# Builder
# ============================================
FROM dependencies AS builder
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --bin icon-api && \
    strip target/release/icon-api && \
    cp target/release/icon-api /usr/local/bin/icon-api

# ============================================
# Development
# ============================================
FROM chef AS development
RUN cargo install cargo-watch --locked
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo chef cook --recipe-path recipe.json

COPY . .
EXPOSE 8080
CMD ["cargo", "watch", "--poll", \
    "-i", "data/*", \
    "-i", "target/*", \
    "-i", "node_modules/*", \
    "-i", "*.log", \
    "-i", ".git/*", \
    "-x", "run --bin icon-api"]


# ============================================
# Runtime
# ============================================
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

RUN groupadd -r appuser && \
    useradd -r -g appuser -d /home/appuser -s /bin/bash appuser && \
    mkdir -p /home/appuser && \
    chown -R appuser:appuser /home/appuser

COPY --from=builder /usr/local/bin/icon-api /usr/local/bin/icon-api
RUN chmod +x /usr/local/bin/icon-api

USER appuser
WORKDIR /home/appuser

EXPOSE 8080
CMD ["icon-api"]
