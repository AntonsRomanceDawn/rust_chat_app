# Build stage
FROM rust:latest as builder

WORKDIR /app

# Copy workspace and server files
COPY Cargo.toml Cargo.toml
COPY server ./server

WORKDIR /app/server

# Build the release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/server/target/release/server /usr/local/bin/server

EXPOSE 3000

CMD ["server"]
