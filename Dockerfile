FROM rust:1.75.0

WORKDIR /app

# dummy main.rs just to install deps
RUN echo 'fn main() { panic!("Dummy Image Called!")}' > ./src/main.rs
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release

COPY ./ ./

# need to break the cargo cache
RUN touch ./src/main.rs
RUN cargo build --release
CMD ["cargo", "run", "--release"]