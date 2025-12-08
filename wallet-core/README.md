# Wallet Core

Low-level wallet and transaction primitives for DuskDS.

This crate sits underneath higher-level libraries like:
- [`rusk-wallet`](https://github.com/dusk-network/rusk/tree/master/rusk-wallet) – Rust wallet library
- [`w3sper.js`](https://github.com/dusk-network/rusk/tree/master/w3sper.js) - JavaScript SDK

> ⚠️ **You probably don't want to use this crate directly.**  
> If you're building a Dusk app, use `rusk-wallet` (Rust) or the JavaScript SDK.
> `dusk-wallet-core` is for protocol / SDK authors.

## What it provides

- Deterministic key derivation from a 64‑byte seed
- Handling of Phoenix notes (ownership, balance, note selection)
- Construction of Moonlight transactions (transfers, staking, deployments)
- A WASM/FFI surface used by higher‑level bindings

## FFI / WASM

This crate exposes a cdylib / WASM interface consumed by the JavaScript SDK and
other bindings. The FFI functions are considered internal and may change
between releases.

If you really need to talk to the FFI layer directly, use the JavaScript SDK or
rusk-wallet as a reference implementation and mirror their usage of the
FFI helpers.

## Documentation

For detailed usage and API examples, refer to the [crate documentation on docs.rs](https://docs.rs/dusk-wallet-core/).

## License

License: MPL-2.0
