# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add error handling for Rues' WebSocket [#3227]
- Add possibility to listen to "connect", "disconnect" and "error" events on the Network instance [#3227]

### Changed

- Changed AddressTransfer to support memo data [#3460]

### Removed

### Fixed

- Fix Rues' connect method not returning the connection promise [#3227]
- Fix Rues' events not being unsubscribed [#3227]
- Fix Rues' event listeners not being removed [#3227]
- Fix "once" events promises not rejecting on errors or disconnections [#3227]
- Fix "subscribe" and "unsubscribe" response body being cancelled after error throwing [#3227]
- Fix `AddressSyncer`'s notes stream hiding a case of error while processing notes [#3227]
- Fix `AddressSyncer`'s BYOB reader not being cancelled after an error [#3227]

## [1.0.0] - 2025-01-15

- First `w3sper.js` release

<!-- ISSUES -->

[#3227]: https://github.com/dusk-network/rusk/issues/3227
[#3460]: https://github.com/dusk-network/rusk/issues/3460

<!-- VERSIONS -->

[Unreleased]: https://github.com/dusk-network/rusk/compare/w3sper-v.0.1.0...HEAD
[1.0.0]: https://github.com/dusk-network/rusk/tree/w3sper-v.0.1.0
