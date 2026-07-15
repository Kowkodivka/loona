FROM rust:1.96.1-slim-bullseye AS builder

WORKDIR /usr/src/loona

COPY Cargo.toml ./
COPY src ./src
COPY migration ./migration

RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 loona
COPY --from=builder /usr/src/loona/target/release/loona /usr/local/bin/loona
USER loona

WORKDIR /home/loona
CMD ["loona"]