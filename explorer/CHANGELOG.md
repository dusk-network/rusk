# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Show the error message for failed transactions [#2220]
- Add tooltip to current and pending stake showing the exact amount [#2363]
- Add `memo` and `isDeploy` fields in transactions [#2362]
- Add `txType` fields in transactions and display it [#2347]
- Add `json` payload to block detail [#2364]

### Changed

- Switch `raw` payload with `json` in transaction details [#2364]
- Change the displayed value for average gas price to lux [#2416]
- Change the blocks table header `FEE` to `GAS` and `AVG` to `AVG PRICE` and `TOTAL` to `USED` [#2416]

### Fixed

- Fix Transactions Fee is not properly computed [#2348]
- Fix shield icons used for tx type [#2389]

## [0.2.0] - 2024-08-26

### Added

- Add DEVNET to the dropdown select on the navbar [#2159]
- Add conditional rendering for layout changes based on screen size [#2061]
- Add accessible name to the gas used progress bar [#2037]
- Add accessible name to the nav bar button on mobile [#2036]
- Implement warning for stale market data [#1892]

### Changed

- Update Statistics Panel separator lines color [#2039]
- Update Statistics Panel labels for clarity [#2034]
- Update font-display to swap for custom fonts to improve performance [#2025]
- Optimize auto re-renders of relative times [#2059]

### Fixed

- Fix Average Fee Paid label [#2057]
- Fix list items alignment on mobile [#2056]

## [0.1.0] - 2024-07-24

### Added

- Create release for explorer module and add changelog file

<!-- ISSUES -->

[#2017]: https://github.com/dusk-network/rusk/issues/2017
[#1892]: https://github.com/dusk-network/rusk/issues/1892
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
[#2220]: https://github.com/dusk-network/rusk/issues/2220
[#2348]: https://github.com/dusk-network/rusk/issues/2348
[#2362]: https://github.com/dusk-network/rusk/issues/2362
[#2363]: https://github.com/dusk-network/rusk/issues/2363
[#2363]: https://github.com/dusk-network/rusk/issues/2347
[#2364]: https://github.com/dusk-network/rusk/issues/2364
[#2389]: https://github.com/dusk-network/rusk/issues/2389

<!-- VERSIONS -->

[Unreleased]: https://github.com/dusk-network/rusk/tree/master/explorer
[0.2.0]: https://github.com/dusk-network/rusk/tree/explorer-0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/explorer-0.1.0
