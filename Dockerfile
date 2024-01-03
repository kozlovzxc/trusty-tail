FROM rust:1.75
WORKDIR /usr/app
COPY . .
RUN cargo build --release
CMD ["cargo", "run", "--release"]