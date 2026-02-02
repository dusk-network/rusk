 # Dusk node library

The Dusk Node functionality crate.

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

### ETL Pipelines

The archive supports configurable ETL pipelines for creating custom indexes from blockchain events.

#### Quick Start

1. Create a `pipelines.json` config file:

```json
{
  "version": 1,
  "pipelines": [
    {
      "id": "my_events",
      "type": "sql_event_table",
      "enabled": true,
      "filter": {
        "topics": ["moonlight", "convert"]
      },
      "sink": {
        "kind": "sqlite_table",
        "table": "idx_my_events",
        "schema": [
          { "name": "topic", "type": "TEXT", "not_null": true },
          { "name": "source", "type": "TEXT" },
          { "name": "data", "type": "BLOB" }
        ],
        "indexes": []
      }
    }
  ]
}
```

2. Add the path to your rusk config:

```toml
[archive]
pipelines_config_path = "/path/to/pipelines.json"
```

#### Pipeline Types

- **`moonlight_builtin`**: Built-in Moonlight transfer indexer (RocksDB)
- **`sql_event_table`**: Generic event filtering to SQLite tables

#### Reserved Columns (auto-added)

`block_height`, `block_hash`, `origin`, `event_ordinal`, `inserted_at`

#### Filter Options

- `contract_ids`: List of contract IDs (hex) to match
- `topics`: List of event topics to match

Both use OR within, AND between (empty = match all).
