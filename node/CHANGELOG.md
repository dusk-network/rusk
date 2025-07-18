# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add support for `TransactionData::Blob`

### Changed

- Change mempool rule to compare gas limit on equal price

## [1.3.0] - 2025-04-17

### Added

- Add transaction serialization check
- Add max transaction size check
- Add active accounts to archive [#3646]
- Add from_block & to_block params to `full_moonlight_history` in archive [#3613]

## [1.2.0] - 2025-03-20

### Added

- Add `create_if_missing` field to `DatabaseOptions`
- Add support for `RUSK_EXT_CHAIN` env

## [1.1.0] - 2025-02-14

### Added

- Add `ledger_txs` to `Ledger` trait and Backend implementation [#3491]

### Fixed

- Change the way the archive synchronizes with the node Acceptor [#3359]

### Changed

- Change deprecated `tempdir` with `tempfile` dependency [#3407]

### Removed

- Removed ArchivistSrv & archivist module [#3359]

## [1.0.1] - 2025-01-23

### Changed

- Change dependency declaration to not require strict equal [#3405]

## [1.0.0] - 2025-01-17

- First `dusk-node` release

<!-- Issues -->
[#3646]: https://github.com/dusk-network/rusk/issues/3646
[#3613]: https://github.com/dusk-network/rusk/issues/3613
[#3491]: https://github.com/dusk-network/rusk/issues/3491
[#3359]: https://github.com/dusk-network/rusk/issues/3359
[#3407]: https://github.com/dusk-network/rusk/issues/3407
[#3405]: https://github.com/dusk-network/rusk/issues/3405

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-node-1.3.0...HEAD
[1.3.0]: https://github.com/dusk-network/rusk/compare/dusk-node-1.2.0...dusk-node-1.3.0
[1.2.0]: https://github.com/dusk-network/rusk/compare/dusk-node-1.1.0...dusk-node-1.2.0
[1.1.0]: https://github.com/dusk-network/rusk/compare/dusk-node-1.0.1...dusk-node-1.1.0
[1.0.1]: https://github.com/dusk-network/rusk/compare/node-1.0.0...dusk-node-1.0.1
[1.0.0]: https://github.com/dusk-network/rusk/tree/node-1.0.0
