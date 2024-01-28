FROM rust:1.75.0

WORKDIR /app
COPY . .
RUN cargo build --release
CMD ["cargo", "run", "--release"]