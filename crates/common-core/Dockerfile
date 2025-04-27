FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build the application
WORKDIR /app
COPY . .
RUN apt-get update --yes && \
    apt-get install --yes protobuf-compiler && \
    cargo build --release --bins --target-dir /app/target/ && \
    sha256sum /app/target/release/exchange /app/target/release/bitcoind-grpc

FROM debian:stable-20250407-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/exchange /usr/local/bin
COPY --from=builder /app/target/release/bitcoind-grpc /usr/local/bin
RUN apt-get update --yes && apt-get install --yes ca-certificates && apt-get clean --yes
ENV RUST_LOG="info"
ENV MACHINE_LOGGING="true"
EXPOSE 3000
CMD ["/usr/local/bin/exchange"]
