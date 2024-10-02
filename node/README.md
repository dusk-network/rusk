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
