# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

### Changed

- Replace legacy event system with RUES [#3425]

### Removed

- Remove version number from app title [#3338]

### Fixed

- Fix wrong owner key shown in provisioners table [#3377]

## [1.0.1] - 2025-01-07

### Fixed

- Fix Owner key (Provisioners page) [#3305]

## [1.0.0] - 2024-12-22

### Added

- Add stake maturity information (Provisioners page) [#3218]
- Add "owner" field to provisioners [#3215]

### Changed

- Change Stake details labels [#3218]
- Change Transaction Type tooltips [#3249]

### Fixed

- Fix inactive stake shown as active on mobile [#3218]

## [0.3.0] - 2024-12-03

### Added

- Add error message for failed transactions [#2220]
- Add tooltips to current and pending stake to show exact amounts [#2363]
- Add `memo` and `isDeploy` fields to transactions [#2362]
- Add `txType` fields in transactions [#2347]
- Add `json` payload to block details [#2364]
- Add decode feature for the `memo` field [#2527]
- Add top node info to StatisticsPanel [#2613]
- Add Provisioners page [#2649]
- Add check for transaction existence in mempool [#2877]

### Changed

- Change `raw` payload to `json` in transaction details [#2364]
- Change average gas price display value to “lux” [#2416]
- Update blocks table headers – `FEE` to `GAS`, `AVG` to `AVG PRICE`, and
  `TOTAL` to `USED` [#2416]
- Update block rewards tooltip information [#2166]
- Hide "Show More" button when error occurs [#2585]
- Update footer layout [#2640]
- Change WorldMap location [#2613]
- Change network info to fetch locally [#2662]
- Update Moonlight icon for visual consistency [#3038]
- Update hosted Explorer links [#3064]

### Fixed

- Fix improper computation of transaction fees [#2348]
- Fix shield icons for transaction types [#2389]
- Fix Gas Used meter behavior when Gas Limit is zero [#2668]
- Fix Cluster Location layout [#3034]

## [0.2.0] - 2024-08-26

### Added

- Add DEVNET option to dropdown menu in the navbar [#2159]
- Add conditional rendering for layout based on screen size [#2061]
- Add accessible name to gas-used progress bar [#2037]
- Add accessible name to navbar button on mobile [#2036]
- Add warning for stale market data [#1892]

### Changed

- Update separator line colors in StatisticsPanel [#2039]
- Update labels in StatisticsPanel for clarity [#2034]
- Update font-display to "swap" for custom fonts, improving performance [#2025]
- Optimize auto re-renders of relative times [#2059]

### Fixed

- Fix “Average Fee Paid” label [#2057]
- Fix list items alignment on mobile [#2056]

## [0.1.0] - 2024-07-24

### Added

- Add initial release for the Explorer module [#2017]

<!-- ISSUES -->

[#1892]: https://github.com/dusk-network/rusk/issues/1892
[#2017]: https://github.com/dusk-network/rusk/issues/2017
[#2025]: https://github.com/dusk-network/rusk/issues/2025
[#2034]: https://github.com/dusk-network/rusk/issues/2034
[#2036]: https://github.com/dusk-network/rusk/issues/2036
[#2037]: https://github.com/dusk-network/rusk/issues/2037
[#2039]: https://github.com/dusk-network/rusk/issues/2039
[#2056]: https://github.com/dusk-network/rusk/issues/2056
[#2057]: https://github.com/dusk-network/rusk/issues/2057
[#2059]: https://github.com/dusk-network/rusk/issues/2059
[#2061]: https://github.com/dusk-network/rusk/issues/2061
[#2159]: https://github.com/dusk-network/rusk/issues/2159
[#2166]: https://github.com/dusk-network/rusk/issues/2166
[#2220]: https://github.com/dusk-network/rusk/issues/2220
[#2347]: https://github.com/dusk-network/rusk/issues/2347
[#2348]: https://github.com/dusk-network/rusk/issues/2348
[#2362]: https://github.com/dusk-network/rusk/issues/2362
[#2363]: https://github.com/dusk-network/rusk/issues/2363
[#2364]: https://github.com/dusk-network/rusk/issues/2364
[#2389]: https://github.com/dusk-network/rusk/issues/2389
[#2416]: https://github.com/dusk-network/rusk/issues/2416
[#2527]: https://github.com/dusk-network/rusk/issues/2527
[#2585]: https://github.com/dusk-network/rusk/issues/2585
[#2613]: https://github.com/dusk-network/rusk/issues/2613
[#2640]: https://github.com/dusk-network/rusk/issues/2640
[#2649]: https://github.com/dusk-network/rusk/issues/2649
[#2662]: https://github.com/dusk-network/rusk/issues/2662
[#2668]: https://github.com/dusk-network/rusk/issues/2668
[#2877]: https://github.com/dusk-network/rusk/issues/2877
[#3034]: https://github.com/dusk-network/rusk/issues/3034
[#3038]: https://github.com/dusk-network/rusk/issues/3038
[#3064]: https://github.com/dusk-network/rusk/issues/3064
[#3215]: https://github.com/dusk-network/rusk/issues/3215
[#3218]: https://github.com/dusk-network/rusk/issues/3218
[#3249]: https://github.com/dusk-network/rusk/issues/3249
[#3305]: https://github.com/dusk-network/rusk/issues/3305
[#3338]: https://github.com/dusk-network/rusk/issues/3338
[#3377]: https://github.com/dusk-network/rusk/issues/3377
[#3425]: https://github.com/dusk-network/rusk/issues/3425

<!-- VERSIONS -->

[Unreleased]: https://github.com/dusk-network/rusk/tree/master/explorer
[1.0.1]: https://github.com/dusk-network/rusk/tree/explorer-v1.0.1
[1.0.0]: https://github.com/dusk-network/rusk/tree/explorer-v1.0.0
[0.3.0]: https://github.com/dusk-network/rusk/tree/explorer-0.3.0
[0.2.0]: https://github.com/dusk-network/rusk/tree/explorer-0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/explorer-0.1.0
