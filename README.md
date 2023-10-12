# exchange-orderbook

# Abstract

exchange-orderbook is an implementation of a cryptocurrency exchange, like Coinbase, Kraken, or Binance. It is written in Rust, uses the Axum web framework, and is backed by a Postgres database.

# Design

The following services can be found in the `docker-compose.yml` file:

* exchange, monolith service providing trading, funding, and account related services via a RESTful webserver API
* NGINX, used for SSL/TLS termination and reverse proxying requests to exchange
* Redis, used as a caching layer and as a message broker between exchange replicas
* Postgres, Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

# Database

The database is a Postgres database. Schema migrations are located in the migrations directory and are managed using the diesel migrate tool. To minimize operational risk—such as system downtime, data loss, or state incoherence between replicas—migrations are executed manually.

mentalfoss@gmail.com
