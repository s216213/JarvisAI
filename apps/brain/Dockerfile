# syntax=docker/dockerfile:1

FROM rust:1.94-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/jarvis-brain /usr/local/bin/jarvis-brain
COPY migrations ./migrations
ENV LISTEN_ADDR=0.0.0.0:8080
EXPOSE 8080
CMD ["jarvis-brain"]
