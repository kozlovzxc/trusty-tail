FROM rust:1.75.0

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=target \
    cargo build --release
CMD ["cargo", "run", "--release"]