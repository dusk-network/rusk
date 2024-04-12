# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

- Update Buttons to match the design system [#1606]

## [0.5.0] - 2024-03-27

### Added

- Add dark mode support [#1466]
- Add autocomplete attribute on the login field [#1533]

### Changed

- Change ribbon's label color [#1598]
- Update font family and letter spacing in buttons, textboxes and selects [#1565]
- Update copy on Reset Wallet while syncing [#1552]
- Trigger the Restore flow if a user tries to access a new wallet [#1535]
- Clear the login info storage on "Reset Wallet" (Settings) [#1551]
- Update `OperationResult` to infer error messages from arbitrary values [#1524]
- Update Tabs to use native scroll behavior [#1320]

### Fixed

- Fix copy button appearance (AddressPicker component) [#1591]
- Fix keydown behavior (AddressPicker component) [#1576]
- Data load and sync UI appearing at the same time [#1545]
- Fix error message overflowing in `ErrorDetails` component [#1547]
- Fix missing focus border on switch component [#1537]

## [0.4.0] - 2024-03-13

### Added

- Add message when no contracts have been enabled [#1317]
- Add `eslint-config-prettier` as explicit dependency [#1509]
- Add format check to CI and `checks` script [#1504]
- Add `vitest-canvas-mock` dependency to replace `canvas` [#1506]
- Add `AppImage` component [#1284]
- Add possibility to serve the web wallet from a sub folder [#1362]

### Changed

- Update to SvelteKit 2, Vite 5 and Vitest 1 [#1284]
- Update all dependencies [#1284]
- Refactor `mockReadableStore` to be not be writable [#1285]
- Refactor beta notice as constant [#1469]
- Refactor `settingsStore` and create readable `gasStore` to store `limitLower`, `limitUpper`, `priceLower` [#1308]
- Refactor add Prettier for formatting and format all files [#1458]

### Removed

- Remove box-shadow from components [#1519]
- Remove orphan dependency `@zerodevx/svelte-toast` [#1509]
- Remove `canvas` dependency [#1506]
- Remove DAT file UI references [#1498]
- Remove `mockDerivedStore` [#1285]
- Remove extraneous code block in MnemonicAuthenticate [#1470]
- Remove `limitLower`, `limitUpper`, `priceLower` from `settingsStore` [#1308]

### Fixed

- Fix layout shift (Balance component) [#1514]
- Fix animation not visible on landing screen [#1501]
- Mismatch between param and JSDoc param's type definition (OperationResult.spec.js) [#1471]
- Fix gas limits update on ENV change [#1308]

## [0.3.0] - 2024-02-28

### Added

- Add Create Wallet flow tests [#1443]
- Add visible version, commit hash and build date [#1441]
- Add Address validation (Transfer flow) [#1377]

### Changed

- Change Get Quote API Endpoint to env variable [#1311]

### Removed

- Remove the use of `checkValidity()` in Send and Stake flow amounts validity checks [#1391]

### Fixed

- Fix typo in routes/welcome/\_\_tests\_\_/page.spec.js [#1445]
- Fix missing whitespace when Transaction list is empty [#1460]

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

[#1284]: https://github.com/dusk-network/rusk/issues/1284
[#1285]: https://github.com/dusk-network/rusk/issues/1285
[#1290]: https://github.com/dusk-network/rusk/issues/1290
[#1308]: https://github.com/dusk-network/rusk/issues/1308
[#1309]: https://github.com/dusk-network/rusk/issues/1309
[#1311]: https://github.com/dusk-network/rusk/issues/1311
[#1317]: https://github.com/dusk-network/rusk/issues/1317
[#1320]: https://github.com/dusk-network/rusk/issues/1320
[#1322]: https://github.com/dusk-network/rusk/issues/1322
[#1323]: https://github.com/dusk-network/rusk/issues/1323
[#1334]: https://github.com/dusk-network/rusk/issues/1334
[#1335]: https://github.com/dusk-network/rusk/issues/1335
[#1343]: https://github.com/dusk-network/rusk/issues/1343
[#1352]: https://github.com/dusk-network/rusk/issues/1352
[#1353]: https://github.com/dusk-network/rusk/issues/1353
[#1354]: https://github.com/dusk-network/rusk/issues/1354
[#1359]: https://github.com/dusk-network/rusk/issues/1359
[#1360]: https://github.com/dusk-network/rusk/issues/1360
[#1361]: https://github.com/dusk-network/rusk/issues/1361
[#1362]: https://github.com/dusk-network/rusk/issues/1362
[#1367]: https://github.com/dusk-network/rusk/issues/1367
[#1373]: https://github.com/dusk-network/rusk/issues/1373
[#1377]: https://github.com/dusk-network/rusk/issues/1377
[#1391]: https://github.com/dusk-network/rusk/issues/1391
[#1398]: https://github.com/dusk-network/rusk/issues/1398
[#1401]: https://github.com/dusk-network/rusk/issues/1401
[#1403]: https://github.com/dusk-network/rusk/issues/1403
[#1410]: https://github.com/dusk-network/rusk/issues/1410
[#1412]: https://github.com/dusk-network/rusk/issues/1412
[#1416]: https://github.com/dusk-network/rusk/issues/1416
[#1417]: https://github.com/dusk-network/rusk/issues/1417
[#1423]: https://github.com/dusk-network/rusk/issues/1423
[#1441]: https://github.com/dusk-network/rusk/issues/1441
[#1443]: https://github.com/dusk-network/rusk/issues/1443
[#1445]: https://github.com/dusk-network/rusk/issues/1445
[#1458]: https://github.com/dusk-network/rusk/issues/1458
[#1460]: https://github.com/dusk-network/rusk/issues/1460
[#1466]: https://github.com/dusk-network/rusk/issues/1466
[#1469]: https://github.com/dusk-network/rusk/issues/1469
[#1470]: https://github.com/dusk-network/rusk/issues/1470
[#1471]: https://github.com/dusk-network/rusk/issues/1471
[#1498]: https://github.com/dusk-network/rusk/issues/1498
[#1501]: https://github.com/dusk-network/rusk/issues/1501
[#1504]: https://github.com/dusk-network/rusk/issues/1504
[#1506]: https://github.com/dusk-network/rusk/issues/1506
[#1509]: https://github.com/dusk-network/rusk/issues/1509
[#1514]: https://github.com/dusk-network/rusk/issues/1514
[#1519]: https://github.com/dusk-network/rusk/issues/1519
[#1524]: https://github.com/dusk-network/rusk/issues/1524
[#1533]: https://github.com/dusk-network/rusk/issues/1533
[#1535]: https://github.com/dusk-network/rusk/issues/1535
[#1537]: https://github.com/dusk-network/rusk/issues/1537
[#1551]: https://github.com/dusk-network/rusk/issues/1551
[#1545]: https://github.com/dusk-network/rusk/issues/1545
[#1547]: https://github.com/dusk-network/rusk/issues/1547
[#1552]: https://github.com/dusk-network/rusk/issues/1552
[#1565]: https://github.com/dusk-network/rusk/issues/1565
[#1576]: https://github.com/dusk-network/rusk/issues/1576
[#1591]: https://github.com/dusk-network/rusk/issues/1591
[#1598]: https://github.com/dusk-network/rusk/issues/1598
[#1606]: https://github.com/dusk-network/rusk/issues/1606

<!-- VERSIONS -->

[Unreleased]: https://github.com/dusk-network/rusk/tree/master/web-wallet
[0.5.0]: https://github.com/dusk-network/rusk/tree/web-wallet-v0.5.0
[0.4.0]: https://github.com/dusk-network/rusk/tree/web-wallet-0.4.0
[0.3.0]: https://github.com/dusk-network/rusk/tree/web-wallet-0.3.0
[0.2.1]: https://github.com/dusk-network/rusk/tree/web-wallet-0.2.1
[0.2.0]: https://github.com/dusk-network/rusk/tree/web-wallet-0.2.0
[0.1.0-beta]: https://github.com/dusk-network/rusk/tree/web-wallet-0.1.0-beta
