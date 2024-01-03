FROM rust:1.75.0

WORKDIR /app

# dummy main.rs just to install deps
RUN mkdir src && \
    echo 'fn main() { panic!("Dummy Image Called!")}' > ./src/main.rs
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release
RUN rm -rf src

COPY ./ ./

RUN cargo build --release
CMD ["cargo", "run", "--release"]