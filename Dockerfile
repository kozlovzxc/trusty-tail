FROM rust:1.75.0

WORKDIR /app

COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo fetch

COPY ./ ./

RUN cargo build --release
CMD ["cargo", "run", "--release"]