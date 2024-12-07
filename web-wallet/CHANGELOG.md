# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

### Changed

- Update Transactions list design [#1922]
- Reword "Staking" header to "Stake" [#3113]
- Upgrade Migration Feature to Use Reown AppKit [#3129]

### Removed

### Fixed

## [0.9.0] - 2024-12-03

### Added

- Add "Support" section under Settings [#3071]
- Add user feedback for "Send" flow validation [#3098]
- Add validation for self-referential transactions [#3099]

### Changed

- Reword "Withdraw Rewards" operation to "Claim Rewards" [#3076]
- Reword "Shield/Unshield" operation to "Allocate" [#3081]

### Removed

- Remove "Shield More Dusk" CTA (Send flow) [#3073]

### Fixed

- Fix "passphrase" terminology usage with "Mnemonic Phrase" [#3069]
- Fix "Stake" flow validation [#3089]

## [0.8.1] - 2024-11-26

### Added

- Add "Reset Wallet" explanatory copy (Settings) [#3061]

### Changed

- Update stake warning's text [#3028]

### Fixed

- Fix UI not scrolling to top after wizard and sub-route navigation [#2997]
- Fix edge case in Dusk to Lux conversion [#3032]
- Fix inconsistent terminology usage for "Mnemonic Phrase" [#3035]
- Fix auto-sync not working after restoring a wallet [#3042]
- Fix application crash on empty amount (Stake Flow) [#3036]
- Fix incorrect fee deduction and negative UI display (Allocate flow) [#3056]
- Fix button hover style in Dashboard Navigation Menu [#2999]
- Fix balance overflowing on small screens [#2994]

## [0.8.0] - 2024-11-19

### Added

- Added gas settings validation on Unstake / Withdraw Rewards flows [#2000]
- Add temporary link to the block explorer on the dashboard [#2882]
- Add Staking-Related Functionality Utilizing w3sper [#3006]
- Add minimum stake amount supplied by w3sper instead of using an env var [#3010]

### Changed

- Update `Stake` to use `Stepper` [#2436]
- Update Mnemonic (Authenticate) Enter key behavior [#2879]
- Enhance Error Handling on Wallet Access Page [#2932]

### Fixed

- Suggested words in the Mnemonic (Authenticate) are accessible using Tab [#2879]
- Enhance Allocate flow on full amount allocation [#2938]
- Broken link in the stake warning [#2990]
- Change "Transaction created" copy [#2991]
- Fix Dashboard navigation menu padding [#3000]

## [0.7.0] - 2024-11-11

### Added

- Added allocation (shield/unshield) page and UI [#2196]
- Add auto-sync every five minutes [#2880]
- Integrate Allocate UI with w3sper's SDK to enable Allocate functionalities [#2920]

### Changed

- Update Balance component [#2863]
- Update UI labels [#2888]

### Fixed

- Fix web-wallet crashing after setting high gas price [#2878]

## [0.6.0] - 2024-11-05

### Added

- Show current block height on Wallet Creation [#1561]
- Add option to sync from a custom block height on Wallet Restoration [#1568]
- Added token migration contract bindings [#2014]
- Add validation for public account ("Send" flow) [#2176]
- Add validation for "Use Max" button on Send / Stake flows [#2310]
- Add Banner component [#2696]
- Add BigIntInput component [#2776]
- Add transaction history feature flag [#2807]

### Changed

- Newly created Wallet does not sync from genesis [#1567]
- Update Buttons to match the design system [#1606]
- Update anchor colors to ensure better accessibility [#1765]
- Update font-display to swap for custom fonts to improve performance [#2026]
- Update `Stepper` component to new design [#2071]
- Update dashboard to use routes instead of Tabs for navigation pattern [#2075]
- Update `Send` to use `Stepper` [#2110]
- Update dashboard by splitting the transfer operations into send and receive operations [#2175]
- Update Balance UI to include an optional UsageIndicator for Moonlight tokens [#2234]
- Restrict mnemonic step input to alphabetical characters (Restore Flow) [#2355]
- Update `Send` to include allocation button [#2420]
- Receive screen design updated, added UI support for displaying shielded/unshielded address [#2421]
- Update ENV variables to the `VITE_FEATURE_*` naming convention [#2434]
- Make address field only vertically resizable (Send flow) [#2435]
- Update textfield input to use the small control sizing [#2498]
- Update GasSettings and related properties to BigInt type [#2778]
- Update to w3sper.js beta [#2821]
- Update sync and balance to use w3sper.js [#2608]
- Update login flows to use w3sper.js [#2460]

### Removed

- Hide staking in the deployed wallet application until w3sper.js supports this.
- Hide transaction history in the deployed application until w3sper.js supports this.

### Fixed

- Fix Receive tab content overflows [#1901]
- Add missing "Soehne Mono" and its @font-face definition [#2071]
- The sync promise should be set to null after aborting a sync [#2118]
- Fix rounding errors in migration amount input [#2303]
- Fix number of leading zeros in migration amount input [#2406]
- Fix Address field invalid state modifier not being applied [#2532]

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
[#1545]: https://github.com/dusk-network/rusk/issues/1545
[#1547]: https://github.com/dusk-network/rusk/issues/1547
[#1551]: https://github.com/dusk-network/rusk/issues/1551
[#1552]: https://github.com/dusk-network/rusk/issues/1552
[#1561]: https://github.com/dusk-network/rusk/issues/1561
[#1565]: https://github.com/dusk-network/rusk/issues/1565
[#1567]: https://github.com/dusk-network/rusk/issues/1567
[#1568]: https://github.com/dusk-network/rusk/issues/1568
[#1576]: https://github.com/dusk-network/rusk/issues/1576
[#1591]: https://github.com/dusk-network/rusk/issues/1591
[#1598]: https://github.com/dusk-network/rusk/issues/1598
[#1606]: https://github.com/dusk-network/rusk/issues/1606
[#1765]: https://github.com/dusk-network/rusk/issues/1765
[#1901]: https://github.com/dusk-network/rusk/issues/1901
[#1922]: https://github.com/dusk-network/rusk/issues/1922
[#2026]: https://github.com/dusk-network/rusk/issues/2026
[#2000]: https://github.com/dusk-network/rusk/issues/2000
[#2014]: https://github.com/dusk-network/rusk/issues/2014
[#2071]: https://github.com/dusk-network/rusk/issues/2071
[#2075]: https://github.com/dusk-network/rusk/issues/2075
[#2110]: https://github.com/dusk-network/rusk/issues/2110
[#2118]: https://github.com/dusk-network/rusk/issues/2118
[#2175]: https://github.com/dusk-network/rusk/issues/2175
[#2176]: https://github.com/dusk-network/rusk/issues/2176
[#2196]: https://github.com/dusk-network/rusk/issues/2196
[#2234]: https://github.com/dusk-network/rusk/issues/2234
[#2303]: https://github.com/dusk-network/rusk/issues/2303
[#2310]: https://github.com/dusk-network/rusk/issues/2310
[#2355]: https://github.com/dusk-network/rusk/issues/2355
[#2406]: https://github.com/dusk-network/rusk/issues/2406
[#2420]: https://github.com/dusk-network/rusk/issues/2420
[#2421]: https://github.com/dusk-network/rusk/issues/2421
[#2434]: https://github.com/dusk-network/rusk/issues/2434
[#2435]: https://github.com/dusk-network/rusk/issues/2435
[#2436]: https://github.com/dusk-network/rusk/issues/2436
[#2460]: https://github.com/dusk-network/rusk/issues/2460
[#2498]: https://github.com/dusk-network/rusk/issues/2498
[#2532]: https://github.com/dusk-network/rusk/issues/2532
[#2696]: https://github.com/dusk-network/rusk/issues/2696
[#2608]: https://github.com/dusk-network/rusk/issues/2608
[#2776]: https://github.com/dusk-network/rusk/issues/2776
[#2778]: https://github.com/dusk-network/rusk/issues/2778
[#2807]: https://github.com/dusk-network/rusk/issues/2807
[#2821]: https://github.com/dusk-network/rusk/issues/2821
[#2863]: https://github.com/dusk-network/rusk/issues/2863
[#2878]: https://github.com/dusk-network/rusk/issues/2878
[#2879]: https://github.com/dusk-network/rusk/issues/2879
[#2880]: https://github.com/dusk-network/rusk/issues/2880
[#2882]: https://github.com/dusk-network/rusk/issues/2882
[#2888]: https://github.com/dusk-network/rusk/issues/2888
[#2920]: https://github.com/dusk-network/rusk/issues/2920
[#2932]: https://github.com/dusk-network/rusk/issues/2932
[#2938]: https://github.com/dusk-network/rusk/issues/2938
[#2990]: https://github.com/dusk-network/rusk/issues/2990
[#2991]: https://github.com/dusk-network/rusk/issues/2991
[#2994]: https://github.com/dusk-network/rusk/issues/2994
[#2997]: https://github.com/dusk-network/rusk/issues/2997
[#2999]: https://github.com/dusk-network/rusk/issues/2999
[#3000]: https://github.com/dusk-network/rusk/issues/3000
[#3006]: https://github.com/dusk-network/rusk/issues/3006
[#3010]: https://github.com/dusk-network/rusk/issues/3010
[#3028]: https://github.com/dusk-network/rusk/issues/3028
[#3032]: https://github.com/dusk-network/rusk/issues/3032
[#3035]: https://github.com/dusk-network/rusk/issues/3035
[#3036]: https://github.com/dusk-network/rusk/issues/3036
[#3042]: https://github.com/dusk-network/rusk/issues/3042
[#3056]: https://github.com/dusk-network/rusk/issues/3056
[#3061]: https://github.com/dusk-network/rusk/issues/3061
[#3069]: https://github.com/dusk-network/rusk/issues/3069
[#3071]: https://github.com/dusk-network/rusk/issues/3071
[#3073]: https://github.com/dusk-network/rusk/issues/3073
[#3076]: https://github.com/dusk-network/rusk/issues/3076
[#3081]: https://github.com/dusk-network/rusk/issues/3081
[#3098]: https://github.com/dusk-network/rusk/issues/3098
[#3099]: https://github.com/dusk-network/rusk/issues/3099
[#3113]: https://github.com/dusk-network/rusk/issues/3113
[#3129]: https://github.com/dusk-network/rusk/issues/3129

<!-- VERSIONS -->

[Unreleased]: https://github.com/dusk-network/rusk/tree/master/web-wallet
[0.9.0]: https://github.com/dusk-network/rusk/tree/web-wallet-v0.9.0
[0.8.1]: https://github.com/dusk-network/rusk/tree/web-wallet-v0.8.1
[0.8.0]: https://github.com/dusk-network/rusk/tree/web-wallet-v0.8.0
[0.7.0]: https://github.com/dusk-network/rusk/tree/web-wallet-v0.7.0
[0.6.0]: https://github.com/dusk-network/rusk/tree/web-wallet-v0.6.0
[0.5.0]: https://github.com/dusk-network/rusk/tree/web-wallet-v0.5.0
[0.4.0]: https://github.com/dusk-network/rusk/tree/web-wallet-0.4.0
[0.3.0]: https://github.com/dusk-network/rusk/tree/web-wallet-0.3.0
[0.2.1]: https://github.com/dusk-network/rusk/tree/web-wallet-0.2.1
[0.2.0]: https://github.com/dusk-network/rusk/tree/web-wallet-0.2.0
[0.1.0-beta]: https://github.com/dusk-network/rusk/tree/web-wallet-0.1.0-beta
