FROM rust:1.75

RUN apt update && apt install -y cron

# Create tmp project
RUN cargo new app
WORKDIR /app
COPY Cargo.toml Cargo.lock .
RUN cargo build --release
# Clean up tmp project
RUN rm src/*.rs

COPY . .
RUN cargo build --release
