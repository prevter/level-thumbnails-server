# syntax=docker/dockerfile:1.7

FROM rust:1.91.1 as base
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder

COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo chef cook --release --recipe-path recipe.json

RUN cargo install --locked sqlx-cli --no-default-features --features rustls,postgres

COPY . .
RUN cargo install --locked --path .

FROM debian:trixie-slim AS runner
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/level-thumbnails-server /usr/local/bin/
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY dist ./dist
COPY migrations ./migrations

RUN mkdir -p /app/logs /app/uploads /app/thumbnails

ENV RUST_LOG=info
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=5 \
    CMD wget -qO- http://127.0.0.1:3000/stats || exit 1

CMD ["level-thumbnails-server"]
