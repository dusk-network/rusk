# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Add running node requirement notice in Staking flow [#1359](https://github.com/dusk-network/rusk/issues/1359)
- Add `fiatPrice` optional property to Balance component [#1323](https://github.com/dusk-network/rusk/issues/1323)
- Add ability to revert words when entering the mnemonic phrase [#1290](https://github.com/dusk-network/rusk/issues/1290)
- Add missing error handling when querying the quote API [#1322](https://github.com/dusk-network/rusk/issues/1322)
- Add gas settings validation to settings page [#1352](https://github.com/dusk-network/rusk/issues/1352)
- Add forced log out on inactive tabs [#1373](https://github.com/dusk-network/rusk/issues/1373)
- Add gas settings validation to block Send and Stake operations if invalid gas settings [#1354](https://github.com/dusk-network/rusk/issues/1354)

### Changed

- Change Holdings component design [#1361](https://github.com/dusk-network/rusk/issues/1361)
- Change `fiatCurrency`, `locale`, `tokenCurrency`, `token` to required properties in Balance component [#1323](https://github.com/dusk-network/rusk/issues/1323)
- Change `package.json` fields to reflect repo change [#1367](https://github.com/dusk-network/rusk/issues/1367)
- Change `walletStore.js` to receive gasPrice and gasLimit when `transfer` , `stake`, `unstake` and `withdrawRewards` are called [#1353](https://github.com/dusk-network/rusk/issues/1353)
- Update deprecated Node actions in CI [#1343](https://github.com/dusk-network/rusk/issues/1343)
- Change `setGasSettings` event to `gasSettings` and include `isValidGas` property in event data [#1354](https://github.com/dusk-network/rusk/issues/1354)

### Removed

- Remove `fiat` property from Balance component [#1323](https://github.com/dusk-network/rusk/issues/1323)
- Remove `gasSettings` store update from `dashboard/+page.svelte.js` [#1353](https://github.com/dusk-network/rusk/issues/1353)

### Fixed
- Fix Changelog to point to issues [#1368](https://github.com/dusk-network/rusk/issues/1368)
- Fix Seed Phrase words size [#1335](https://github.com/dusk-network/rusk/issues/1335)
- Fix colors on red background [#1334](https://github.com/dusk-network/rusk/issues/1334)
- Fix Transactions table design [#1309](https://github.com/dusk-network/rusk/issues/1309)

## [0.1.0-beta] - 2024-02-02

### Added

- Add initial commit

<!-- ISSUES -->

<!-- VERSIONS -->
[Unreleased]: https://github.com/dusk-network/rusk/compare/web-wallet-0.1.0-beta...HEAD
[0.1.0-beta]: https://github.com/dusk-network/rusk/tree/web-wallet-0.1.0-beta
