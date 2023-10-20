# exchange-orderbook

# Abstract

exchange-orderbook is an implementation of a spot exchange; like Coinbase, Kraken, or Binance supporting Bitcoin (BTC) and Ether (ETH) (NB: crypto is just an implementation detail here so dont get hung up on it you can replace it with any other asset.)

Written in Rust, using the Axum web framework, and backed by Postgres.

# Service Architecture

The following services can be found in the `docker-compose.yml` file:

* exchange, monolith service providing trading, funding, and account related services via a RESTful webserver API
* NGINX, used for SSL/TLS termination and reverse proxying requests to exchange
* Redis, used as a caching layer and as a message broker between exchange replicas
* Postgres, Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

# Database

The database is a Postgres database. Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.


# Low-Level Notes

## Orderbook

The Orderbook is a key component that leaves room for interpretation when it comes to implementation. Here our orderbook type is very simple: It contains two vectors: `bids` and `asks`, which hold price levels sorted in ascending order. Each price level itself houses a not-explicitly-sorted (read: orders at an index with a lower value than those of a higher index came earlier as .push adds to the right hand side) list of orders.

Here is some code to illustrate the design:

```rs
pub struct Orderbook {
    bids: MultiplePriceLevels,
    asks: MultiplePriceLevels,
}

pub struct MultiplePriceLevels {
    inner: Vec<PriceLevel>, // NB: we use TinyVec here
}

pub struct PriceLevel {
    price: NonZeroU32,
    memo_seq: u32,
    inner: Vec<Order>, // NB: we also use TinyVec here
}
```

- `MultiplePriceLevels`: A vector that holds price levels.
- `PriceLevel`: Contains the price, a memo number generator, and the orders for that price level.

An order-index type is employed for finding an order. The respective price level is located using binary search, while the order within that level is found via a linear scan.

```rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderIndex {
    side: OrderSide,  // bid (buy) or ask (sell)
    price: NonZeroU32,  // price level (non-zero)
    memo: u32, // some unique identifier for the order (e.g. monotonically increasing sequence number)
}
```

The design aims to minimize the cost of pointer indirection by opting for
linear scans when accessing orders. also we use [tinyvec]
for the inner vectors because I am a big degenerate fan of premature
optimization and the excuse I am going to use is that this improves cache
locality and avoids premature heap allocations.

A contiguous array storage could theoretically improve cache hits but then we have
to care about the memory management of the array.
if we want to avoid reallocations e.g. by re-using cells in the array that
are no longer in use then we need to keep track of which cells are in use and
which are not (for example with a tombstone bit.) and dont get me started on
how to efficiently search for orders in this setup. This is a lot of complexity
for a small gain, so we opt for the simpler design.

mentalfoss@gmail.com

[tinyvec]: https://docs.rs/tinyvec
