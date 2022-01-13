# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2021-01-12

### Added

- Serialization and deserialization of transactions.
- Temporary deterministic wallet generation.
- Implementation of `get_balance` and `public_spend_key`.
- Preliminary implementation of `create_transfer_tx`.
- Expose `NodeClient` and `Store` through FFI.
- Define FFI and compile it only for WASM.
- `NodeClient` and `Store` traits encapsulating host dependant functionality.
- Initial commit.

[Unreleased]: https://github.com/dusk-network/wallet-core/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/wallet-core/releases/tag/v0.1.0
