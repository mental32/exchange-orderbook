# exchange-orderbook

# Abstract

exchange-orderbook is an implementation of a spot exchange; like Coinbase, Kraken, or Binance supporting Bitcoin (BTC) and Ether (ETH) (NB: crypto is just an implementation detail here that happened to be easy to implement.)

Written in Rust, using the Axum web framework, and backed by Postgres.

# Quick Start

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Diesel CLI](https://diesel.rs/guides/getting-started/)
- [Protobuf Compiler](https://grpc.io/docs/protoc-installation/)

## Building

To only build the exchange and grpc-proxy executable binaries:

```
cargo build --release --bins
```

They should be found in `target/release/exchange` and `target/release/bitcoind-grpc-proxy` respectively.

## Running

To run the exchange and grpc-proxy it is recommended to use docker-compose:

```
docker-compose up -d --build
```

This will start the exchange, nginx, redis, postgres, and bitcoind and bitcoind-grpc-proxy services.

> bitcoind initial sync
>
> It will take some time for the bitcoin core full node to sync with the testnet, this is only a couple hours to a day
> depending on your internet connection. If you dont want to wait you can always configure the bitcoind service to use the
> regtest network instead of the testnet.
>
> disable or comment out `testnet=1` in `etc/bitcoind/bitcoin.conf` and set `regtest=1` instead.

## Testing

To run built-in unit and integration tests:

```
cargo test
```

## Interacting

You can interact with the exchange with a TUI client:

```
cargo run --bin exchange-tui
```

# Service Architecture

The following services can be found in the `docker-compose.yml` file:

* exchange, monolith service providing trading, funding, and account related services via a RESTful webserver API
* NGINX, used for SSL/TLS termination and reverse proxying requests to exchange
* Redis, used as a caching layer and as a message broker between exchange replicas
* Postgres, Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

Also the following services have been added:

* bitcoind, a bitcoin core node used for generating addresses and streaming transactions configured to use the testnet
* bitcoind-grpc-proxy, a grpc server to proxy requests to bitcoind written because the bitcoin core jsonrpc api is disturbing and a well-typed grpc api is much nicer to work with

## Exchange

The exchange service is a single-process service providing trading, funding, and account related services via a RESTful webserver API.

## NGINX

NGINX serves as the web server and reverse proxy:

- **SSL/TLS Termination**: Handles encryption for secure traffic.
- **Reverse Proxy**: Routes traffic to `exchange` API instances.
- **Future Capabilities**: Potential rate limiting implementation.

## Redis

Redis is used as a caching layer and as a message substrate between exchange replicas. The cache is used to store session
data and other ephemeral data for the exchange API.

## Postgres

The database is a Postgres database. Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

## bitcoind (Bitcoin Core)

The best way to interact with the Bitcoin network is to run a full node. It will index, verify, track, and manage transactions and wallets. It will also allow us to generate addresses and off-ramp BTC to users.

## bitcoind-grpc-proxy

The currently existing jsonrpc and bitcoin rpc crates are not very well made, poorly documented, and impose unwanted dependencies on the project. The bitcoin core code itself is a type of C/C++ i can't navigate very well. So I wrote a grpc proxy to expose a well-typed interface to the exchange service while dealing with the bitcoin core jsonrpc interactions in a separate process.

mentalfoss@gmail.com

[tinyvec]: https://docs.rs/tinyvec
[event sourcing]: https://microservices.io/patterns/data/event-sourcing.html
