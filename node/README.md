<div align="center">

# `🌐 Dusk Node`

> Chain node implementation — networking, mempool, storage, and synchronization
</div>

## Overview

Dusk Node is the chain layer of the Dusk blockchain. It manages peer-to-peer networking, transaction pooling, persistent ledger storage, and chain synchronization. It integrates the [`consensus`](../consensus/) engine and is orchestrated by [`rusk`](../rusk/).

## Key Modules

| Module | Description |
|--------|-------------|
| `database` | RocksDB-based ledger storage |
| `mempool` | Transaction pool management and eviction |
| `network` | Peer communication via Kadcast |
| `databroker` | Block and transaction serving to peers |
| `telemetry` | Metrics collection and reporting |

## Features

| Feature | Description |
|---------|-------------|
| `archive` | SQLite-based historical data indexing (see below) |
| `network-trace` | Network-level debug tracing |

## Related Crates

- [`node-data`](../node-data/) — ledger and message types consumed by the node
- [`dusk-consensus`](../consensus/) — consensus engine driving block production
- [`dusk-core`](../core/) — cryptographic signatures and transaction types
- [`rusk`](../rusk/) — orchestrates the node as part of the full binary

## Archive feature

The current archive makes use of SQLite and SQLx in [offline mode](https://docs.rs/sqlx/latest/sqlx/macro.query.html#offline-mode).

Installing sqlx-cli with ``cargo install sqlx-cli --features openssl-vendored``

### Offline mode

**If the queries don't change, nothing needs to be done.**

If queries do change, you need to set a database env var and update the offline .sqlx queries folder.

This can be done through:
1. ``export DATABASE_URL=sqlite:///tmp/temp.sqlite3``
2. ``cargo sqlx prepare -- --all-targets --all-features``

### Non offline mode

In order for the `sqlx::query` macro to successfully expand during compile time checks, a database must exist beforehand if not run in offline mode.

This can be done through:
1. Set DATABASE_URL or create .env file with ``DATABASE_URL=sqlite:///tmp/temp.sqlite3``
2. Create a db with ``sqlx database create`` 
3. Run the migrations with ``sqlx migrate run``

> NB: You need to be in the /node folder of this project for sqlx to detect the migrations folder
