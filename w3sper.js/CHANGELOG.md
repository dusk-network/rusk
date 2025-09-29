# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add `deposit` field to the transaction builder to deposit funds into contracts, allowing explicit values to be set instead of default `0n` [#3868]
- Add data-driver runtime (`src/data-driver/loader.js`, `registry.js`, `mod.js`) for JSON <-> RKYV serialization and WASM driver loading [#3876]
- Add minimal `Contract` API with `call`, and `tx` methods for driver-backed reads and writes [#3876]
- Add first-class decoded contract events with `contract.events.<event>.once()/on()` using RUES and automatic driver-based event decoding [#3877]

### Changed

### Removed

### Fixed

## [1.2.0] - 2025-09-17

### Added

- Add possibility to retrieve the unshielded transaction history [#3427]
- Add support for transaction payload carrying contract call data [#3750]

### Changed

- Change `wallet-core` URL to a concrete versioned URL [#3850]

### Removed

### Fixed

## [1.1.0] - 2025-03-26

### Added

- Add error handling for Rues' WebSocket [#3227]
- Add possibility to listen to "connect", "disconnect" and "error" events on the Network instance [#3227]
- Add keep-alive behaviour in Rues' websocket [#3582]

### Changed

- Changed AddressTransfer to support memo data [#3460]
- Prevent new WebSocket creation if a connection is still active [#3568]

### Removed

### Fixed

- Fix Rues' connect method not returning the connection promise [#3227]
- Fix Rues' events not being unsubscribed [#3227]
- Fix Rues' event listeners not being removed [#3227]
- Fix "once" events promises not rejecting on errors or disconnections [#3227]
- Fix "subscribe" and "unsubscribe" response body being cancelled after error throwing [#3227]
- Fix `AddressSyncer`'s notes stream hiding a case of error while processing notes [#3227]
- Fix `AddressSyncer`'s BYOB reader not being cancelled after an error [#3227]
- Fix Rues not dispatching a "disconnect" event when the socket closes on its own [#3568]
- Fix AbortController's abort on Rues events not triggering unsubscription from server [#3582]

## [1.0.0] - 2025-01-15

- First `w3sper.js` release

<!-- ISSUES -->

[#3227]: https://github.com/dusk-network/rusk/issues/3227
[#3460]: https://github.com/dusk-network/rusk/issues/3460
[#3568]: https://github.com/dusk-network/rusk/issues/3568
[#3582]: https://github.com/dusk-network/rusk/issues/3582
[#3427]: https://github.com/dusk-network/rusk/issues/3427
[#3750]: https://github.com/dusk-network/rusk/issues/3750
[#3850]: https://github.com/dusk-network/rusk/issues/3850
[#3868]: https://github.com/dusk-network/rusk/issues/3868
[#3876]: https://github.com/dusk-network/rusk/issues/3876
[#3877]: https://github.com/dusk-network/rusk/issues/3877

<!-- VERSIONS -->

[Unreleased]: https://github.com/dusk-network/rusk/compare/w3sper-v.0.1.0...HEAD
[1.2.0]: https://github.com/dusk-network/rusk/tree/w3sper-v.1.2.0
[1.1.0]: https://github.com/dusk-network/rusk/tree/w3sper-v.1.1.0
[1.0.0]: https://github.com/dusk-network/rusk/tree/w3sper-v.0.1.0
