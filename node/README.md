 # Dusk node library

The Dusk Node functionality crate.

## Archive feature

The current archive makes use of SQLite and SQLx.

In order for the `sqlx::query` macro to successfully expand during compile time checks, a database must exist beforehand.

This can be done through:
1. Installing sqlx-cli with ``cargo install sqlx-cli --features openssl-vendored``
2. Create a db with ``sqlx database create`` (this takes the db info out of .env if no DATABASE_URL is set)
3. Run the migrations with ``sqlx migrate run``
