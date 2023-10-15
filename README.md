# exchange-orderbook

# Abstract

exchange-orderbook is an implementation of a cryptocurrency exchange, like Coinbase, Kraken, or Binance. It is written in Rust, uses the Axum web framework, and is backed by a Postgres database.

# Service Architecture

The following services can be found in the `docker-compose.yml` file:

* exchange, monolith service providing trading, funding, and account related services via a RESTful webserver API
* NGINX, used for SSL/TLS termination and reverse proxying requests to exchange
* Redis, used as a caching layer and as a message broker between exchange replicas
* Postgres, Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

# Low-Level Details: Orderbook

The orderbook stores orders in one of two vectors, bids or asks. Each vector
contains price levels, which are sorted in ascending order (lowest to highest).
each price level contains a list of orders, which are internally unsorted and
thus sorted in the order they were received.

```rust
pub struct Orderbook {
    bids: MultiplePriceLevels,
    asks: MultiplePriceLevels,
}

/// The threshold at which the [`MultiplePriceLevels`] will switch from using array storage to heap storage.
pub const MULTIPLE_PRICE_LEVEL_INNER_CAPACITY: usize = 128;

/// Stores multiple price levels in a contiguous vector.
pub struct MultiplePriceLevels {
    inner: TinyVec<[PriceLevel; MULTIPLE_PRICE_LEVEL_INNER_CAPACITY]>,
}


/// The threshold at which the [`PriceLevel`] will switch from using array storage to heap storage.
const PRICE_LEVEL_INNER_CAPACITY: usize = 64;

/// The inner data structure for a [`MultiplePriceLevels`].
pub struct PriceLevel {
    /// The price of the orders in this price level.
    price: u32,
    /// The sequence number generator for the next order to be added to this price level.
    memo_seq: u32,
    /// The inner data structure storing the orders in this price level.
    inner: TinyVec<[Order; PRICE_LEVEL_INNER_CAPACITY]>,
}
```

Accessing an order in the book is done with an order-index type. The price-level is resolved using a binary search, and the order is resolved using a linear scan. This approach was chosen as a cost-benefit tradeoff between the cost of pointer-indirection and the cost of overhead to maintain an appropriate data structure.

We care about the cost of pointer-indirection
so ideally we would store the orders in a contiguous array. However; we would have to design a storage solution that avoids reallocation keeping orders in the same memory location and the index valid, the overhead to maintain this is not worth the performance gain. Additionally, we would have to pay a cost of continually memory copying ranges of the vector when reallocating when inserting orders midway into the structure.

The above notes are super cool to implement for a high-performance book that optimizes for cache hits and compactness, but for this project, we are not concerned with that. We are concerned with correctness and simplicity. The approach chosen is simple and correct.

# Database

The database is a Postgres database. Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

mentalfoss@gmail.com
