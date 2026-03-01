# syntax=docker/dockerfile:1.7

FROM rust:1.93-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY README.md LICENSE ./

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/owui-lint /usr/local/bin/owui-lint

ENTRYPOINT ["owui-lint"]
CMD ["--help"]
