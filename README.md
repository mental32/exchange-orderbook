# exchange-orderbook

# Abstract

exchange-orderbook is an implementation of a spot exchange; like Coinbase, Kraken, or Binance supporting Bitcoin (BTC) and Ether (ETH) (NB: crypto is just an implementation detail here that happened to be easy to implement.)

Written in Rust, using the Axum web framework, and backed by Postgres.

# Quick Start

> disclaimer: work-in-progress, a lot of things are not yet finished. see the progress checklist at the end of this file.

## Prerequisites

Please ensure you have the following programs installed on the system:

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Diesel CLI](https://diesel.rs/guides/getting-started/)
- [Protobuf Compiler](https://grpc.io/docs/protoc-installation/)

## Compiling from source (without docker)

To only build the exchange and grpc-proxy executable binaries:

```
cargo build --release --bins
```

They should be found in `target/release/exchange` and `target/release/bitcoind-grpc-proxy` respectively.

## Running (with docker)

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


<hr>

# Interacting with the exachange

Using docker-compose to run the exchange you can then interact using the website at `https://localhost:80`

<hr>


# Service Architecture

The following services can be found in the `docker-compose.yml` file:

* exchange, monolith service providing trading, funding, and account related services via a RESTful webserver API
* NGINX, used for SSL/TLS termination and reverse proxying requests to exchange
* Redis, used as a caching layer and as a message broker between exchange replicas
* Postgres, Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

Also the following services have been added:

* bitcoind, a bitcoin core node used for generating addresses and streaming transactions configured to use the testnet
* bitcoind-grpc-proxy, a grpc server to proxy requests to bitcoind written because the bitcoin core jsonrpc api is disturbing and a well-typed grpc api is much nicer to work with

### Exchange

The exchange service is a single-process service providing trading, funding, and account related services via a RESTful webserver API.

### NGINX

NGINX serves as the web server and reverse proxy:

- **SSL/TLS Termination**: Handles encryption for secure traffic.
- **Reverse Proxy**: Routes traffic to `exchange` API instances.
- **Future Capabilities**: Potential rate limiting implementation.

### Redis

Redis is used as a caching layer and as a message substrate between exchange replicas. The cache is used to store session
data and other ephemeral data for the exchange API.

### Postgres

The database is a Postgres database. Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

### bitcoind (Bitcoin Core)

The best way to interact with the Bitcoin network is to run a full node. It will index, verify, track, and manage transactions and wallets. It will also allow us to generate addresses and off-ramp BTC to users.

### bitcoind-grpc-proxy

The currently existing jsonrpc and bitcoin rpc crates are not very well made, poorly documented, and impose unwanted dependencies on the project. The bitcoin core code itself is a type of C/C++ i can't navigate very well. So I wrote a grpc proxy to expose a well-typed interface to the exchange service while dealing with the bitcoin core jsonrpc interactions in a separate process.

# Progress Checklist

- [ ] ETH funding
    - [ ] connect to a node and stream Tx confirmations
    - [ ] use a local-wallet abstraction to sign messages and generate deposit addresses
- [ ] BTC funding
    - [ ] bitcoincore grpc proxy:
        - [ ] rpc method: create an address
    - [ ] create entry in tx_journal for user debit
- [ ] settlements
    - [ ] generate withdrawal addresses
    - [ ] crypto withdrawal flow
    - [ ] bundle individual settlements into one TXs
- [ ] Database Schema
    - [ ] table for deposit addresses
        - [x] BTC
        - [ ] ETH
    - [ ] table for withdrawal addresses
    - [x] table for user accounts
    - [x] append-only table of order events for event sourcing
    - [x] accounting tables for double-entry bookkeeping
- [ ] Trading Engine
    - [x] orderbook with price-level groupings
    - [x] order-uuid generation & resolves to orderbook index
    - [ ] Place Order
        - [ ] self-trade-protection
        - [ ] time-in-force
            - [x] GTC
            - [ ] GTD
            - [x] FOK
            - [x] IOC
        - [ ] iceberg orders
        - [x] market type orders
        - [x] limit type orders
    - [ ] Cancel Order
    - [ ] Amend Order
- [ ] exchange-api
    - [x] session-token auth check for `/trade`
    - [x] JSON content-type request body extractor
    - [ ] msgpack content-type request body extractor
    - [ ] user endpoints
        - [ ] `/trade/` relm
            - [x] POST add order
            - [x] DELETE cancel order
            - [ ] PATCH amend order
        - [ ] `/session` relm
            - [ ] POST create session
        - [ ] `/public` relm
            - [x] get server time
            - [ ] get orderbook
            - [ ] get asset pairs
            - [ ] get market data
    - [ ] admin endpoints
        - [ ] get trade history
        - [ ] get user balance
        - [ ] resume/suspend user account
    - [ ] operator endpoints
        - [ ] process shutdown (immediate)
        - [ ] process shutdown (wind-down)
        - [ ] TE resume/suspend


mentalfoss@gmail.com

[tinyvec]: https://docs.rs/tinyvec
[event sourcing]: https://microservices.io/patterns/data/event-sourcing.html
