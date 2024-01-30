FROM rust:1.75.0

# Create tmp project
RUN cargo new app
WORKDIR /app
# Overwrite tmp project deps
COPY Cargo.toml Cargo.lock .
# Install & build deps
RUN cargo build --release
# Clean up tmp project
RUN rm src/*.rs

COPY . .
RUN cargo build --release
CMD ["cargo", "run", "--release"]