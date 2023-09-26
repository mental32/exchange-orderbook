FROM rust as builder
WORKDIR /app
COPY ./ ./
RUN cargo fetch
RUN apt-get update --yes && apt-get install --yes protobuf-compiler
RUN cargo build --release --bin exchange --target-dir /app/target/ && sha256sum /app/target/release/exchange

FROM debian:stable-20230919-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/exchange /usr/local/bin
RUN apt-get update --yes && apt-get install --yes ca-certificates && apt-get clean --yes
ENV RUST_LOG "info"
ENV MACHINE_LOGGING "true"
EXPOSE 3000
CMD ["/usr/local/bin/exchange"]
