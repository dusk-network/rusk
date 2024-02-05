# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add `fiatPrice` optional property to Balance component [#1331](https://github.com/dusk-network/rusk/pull/1331)
- Add ability to revert words when entering the mnemonic phrase [#1333](https://github.com/dusk-network/rusk/pull/1333)
- Add missing error handling when querying the quote API [#1344](https://github.com/dusk-network/rusk/pull/1334)

### Changed

- Change `fiatCurrency`, `locale`, `tokenCurrency`, `token` to required properties in Balance component [#1331](https://github.com/dusk-network/rusk/pull/1331)

### Removed

- Remove `fiat` property from Balance component [#1331](https://github.com/dusk-network/rusk/pull/1331)

### Fixed

- Fix Seed Phrase words size [#1337](https://github.com/dusk-network/rusk/pull/1337)
- Fix colors on red background [#1336](https://github.com/dusk-network/rusk/pull/1336)
- Fix Transactions table design [#1330](https://github.com/dusk-network/rusk/pull/1330)

## [0.1.0-beta] - 2024-02-02

### Added

- Add initial commit

<!-- ISSUES -->

<!-- VERSIONS -->
[Unreleased]: https://github.com/dusk-network/rusk/compare/web-wallet-0.1.0-beta...HEAD
[0.1.0-beta]: https://github.com/dusk-network/rusk/tree/web-wallet-0.1.0-beta
