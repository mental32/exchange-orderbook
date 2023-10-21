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

## Trading Engine

The term "trading engine" is a little ambiguous so to break it down, a trading engine is simply an abstraction around a
matching engine and the owner responsible for mutation of the orderbooks (books) that also can perform checks for risk
analysis depending on, of course, how complex the underlying instrument is

The trading engine's (TE) call graph is, assuming you have lots of users, _the_ hot path of the entire exchange
operation. It is also crack cocaine for engineers who love optimization challenges because in financial markets
a microsecond can be an eternity so time is literally money which makes latency is the most important factor
after correctness.

The state of the orderbook must be coherent always since it can represent an eye watering amount of money in the system
consequently whatever manages the mutation of the orderbook must be correct and capable of unwinding or never commiting
change that causes a loss of coherency in the books.

But specifically in this code the TE is:

1. Responsible for managing orders into the system after ingress.
2. Resolving incoming orders against the resting orders across all the books and
3. Executing triggers to cause external updates, e.g. writes to the database to commit results of an operation, notify users of fills

The design is reminicent of the actor in an actor model of concurrency. Messages are sent over a bounded channel and
are then processed by the TE event loop which is running on separate thread. the structure of the event loop is:

```rs
loop {
    let message = tokio::sync::mpsc::Receiver::blocking_recv();

    match message {
        // switch on a message
        // - rewind, suspend, resume, shutdown
        // - place-order, cancel-order, amend-order
    }

    // execute an appropriate trigger
    // boils down to a function call to send a message back to the main task to perform some side effects
}
```

The TE performs no side-effects, everthing happens by receiving and sending messages to the state machine running in
a separate thread. there is no internal randomness in the system and its output is totally deterministic given the input
is streamed in at the same order and integrity as it was initially.

This makes it a fantastic candidate for [event sourcing.]

<!-- TE perfoming "no side effects" is not strictly true in a Haskell sense as it does mutate memory in-place
but this is very reasonable design choice to excuse away -->

### Time Travel

The coolest thing about the design of the TE is what I like to call "time travel".

For example. consider there might be some bug in the trading engine logic that is dicovered by some business message
like place-order or amend-order causing the engine to panic and explode. obviously this is a problem since no matter
which replica processes the message it will always deterministically crash the TE. This is not good.

But assuming we have the ability (spoiler alert: "we do.") to:

1. Detect when the TE thread panics and crashes
2. Design the TE so that:
    - we can determine, for any input, what the inverse operations are
    - run the inverse operations effectively an "undo" operation continously until we reach a state that we had at a
      previous point in time.

All we need is a frankly small amount of code to recover from bad inputs or environmental failures requiring us to
abort processing the input entirely, a success story for reproducability and fault tolerance!

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
[event sourcing]: https://microservices.io/patterns/data/event-sourcing.html
