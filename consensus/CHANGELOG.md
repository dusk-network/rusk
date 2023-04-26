# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add CHANGELOG. [#54]
- Add `get_mempool_txs`. [#47]
- Add node-data crate. [#44]
- Add description for consensus phases. [#38]

### Changed

- Expose `verify_step_votes`. [#50]

### Fixed

- Fix `VoteSetTooSmall` in consensus accumulator. [#53]
- Fix DUSK base value. [#51]
- Fix compatibility issues between latest node-data crate and consensus. [#44]

## [0.1.0] - 2023-01-05

- First `consensus` release

<!-- ISSUES -->
[#54]: https://github.com/dusk-network/consensus/issues/54
[#53]: https://github.com/dusk-network/consensus/issues/53
[#51]: https://github.com/dusk-network/consensus/issues/51
[#50]: https://github.com/dusk-network/consensus/issues/50
[#47]: https://github.com/dusk-network/consensus/issues/47
[#44]: https://github.com/dusk-network/consensus/issues/44
[#42]: https://github.com/dusk-network/consensus/issues/42
[#38]: https://github.com/dusk-network/consensus/issues/38

<!-- VERSIONS -->
[Unreleased]: https://github.com/dusk-network/piecrust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/piecrust/releases/tag/v0.1.0
