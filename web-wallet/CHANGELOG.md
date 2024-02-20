# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Removed

- Remove the use of `checkValidity()` in Send and Stake flow amounts validity checks [#1391]

### Fixed

## [0.2.1] - 2024-02-20

### Added

- Add wallet restore flow tests [#1416]
- Add missing login flow tests [#1423]

### Fixed

- Fix restore flow allowing invalid mnemonic to be used to log in [#1416]
- Fix can't unlock the wallet with upper case words [#1417]

## [0.2.0] - 2024-02-15

### Added

- Add running node requirement notice in Staking flow [#1359]
- Add `fiatPrice` optional property to Balance component [#1323]
- Add ability to revert words when entering the mnemonic phrase [#1290]
- Add missing error handling when querying the quote API [#1322]
- Add gas settings validation to settings page [#1352]
- Add forced log out on inactive tabs [#1373]
- Add gas settings validation to block Send and Stake operations if invalid gas settings [#1354]
- Add abortable sync [#1401]
- Add `existing wallet notice` to wallet create, restore and login flows [#1360]
- Add `userId` value to localStorage preferences object during wallet create and restore [#1360]

### Changed

- Change Holdings component design [#1361]
- Change `fiatCurrency`, `locale`, `tokenCurrency`, `token` to required properties in Balance component [#1323]
- Change `package.json` fields to reflect repo change [#1367]
- Change `walletStore.js` to receive gasPrice and gasLimit when `transfer` , `stake`, `unstake` and `withdrawRewards` are called [#1353]
- Update deprecated Node actions in CI [#1343]
- Change `setGasSettings` event to `gasSettings` and include `isValidGas` property in event data [#1354]
- Change "withdraw stake" label to "unstake" [#1403]
- Change logout flow to abort a sync if in progress [#1401]
- Update dusk-wallet-js to from 0.3.2 to 0.4.2 [#1401]

### Removed

- Remove `fiat` property from Balance component [#1323]
- Remove `gasSettings` store update from `dashboard/+page.svelte.js` [#1353]

### Fixed

- Fix Transactions table remains hidden for some screen resolutions [#1412]
- Fix Stake button is always disabled [#1410]
- Fix wizard progression on Stake flow [#1398]
- Fix Seed Phrase words size [#1335]
- Fix colors on red background [#1334]
- Fix Transactions table design [#1309]

## [0.1.0-beta] - 2024-02-02

### Added

- Add initial commit

<!-- ISSUES -->
[#1359]: https://github.com/dusk-network/rusk/issues/1359
[#1323]: https://github.com/dusk-network/rusk/issues/1323
[#1290]: https://github.com/dusk-network/rusk/issues/1290
[#1322]: https://github.com/dusk-network/rusk/issues/1322
[#1352]: https://github.com/dusk-network/rusk/issues/1352
[#1373]: https://github.com/dusk-network/rusk/issues/1373
[#1354]: https://github.com/dusk-network/rusk/issues/1354
[#1401]: https://github.com/dusk-network/rusk/issues/1401
[#1360]: https://github.com/dusk-network/rusk/issues/1360
[#1361]: https://github.com/dusk-network/rusk/issues/1361
[#1367]: https://github.com/dusk-network/rusk/issues/1367
[#1353]: https://github.com/dusk-network/rusk/issues/1353
[#1343]: https://github.com/dusk-network/rusk/issues/1343
[#1403]: https://github.com/dusk-network/rusk/issues/1403
[#1412]: https://github.com/dusk-network/rusk/issues/1412
[#1410]: https://github.com/dusk-network/rusk/issues/1410
[#1398]: https://github.com/dusk-network/rusk/issues/1398
[#1335]: https://github.com/dusk-network/rusk/issues/1335
[#1334]: https://github.com/dusk-network/rusk/issues/1334
[#1309]: https://github.com/dusk-network/rusk/issues/1309
[#1416]: https://github.com/dusk-network/rusk/issues/1416
[#1423]: https://github.com/dusk-network/rusk/issues/1423
[#1391]: https://github.com/dusk-network/rusk/issues/1391
[#1417]: https://github.com/dusk-network/rusk/issues/1417

<!-- VERSIONS -->
[Unreleased]: https://github.com/dusk-network/rusk/tree/master/web-wallet
[0.2.1]: https://github.com/dusk-network/rusk/tree/web-wallet-0.2.1
[0.2.0]: https://github.com/dusk-network/rusk/tree/web-wallet-0.2.0
[0.1.0-beta]: https://github.com/dusk-network/rusk/tree/web-wallet-0.1.0-beta
