# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Add setup/teardown system for tests [#292]
- Add `dusk-jubjub v0.10` to deps [#292]
- Add `async-stream` to deps [#292]
- Add `test-context v0.1` to dev-dependencies [#292]
- Add `async-trait v0.1` to dev-dependencies [#292]
- Add `RUSK_PROFILE_PATH` env variable check in `build.rs` [#307]

### Changed

- Update build system improving Circuit keys caching [#290]
- Update `tokio` to `v1.6` [#292]
- Update `tonic` from `0.3` to `0.4` [#292] 
- Update `prost` from `0.6` to `0.7` [#292]
- Change `tower` to be a dev-dependency [#292]
- Refactor `unix` modules from `tests` and `bin` [#292]

### Fixed

- Fix dusk-bytes encoding issues [#292]
- Fix score generation module/service [#292]

### Removed

- Remove `bincode` since is unused [#292]
- Remove `default-feats = false` from `dusk-plonk` [#292]
- Remove `thiserror` from dependencies [#292]

## [0.1.0] - 2021-02-19

### Added

- Add Rusk server impl
- Add BlindBid service
- Add Pki service
- Add Echo service
- Add encoding module
- Add clap cli interface
- Add linking between Rusk and Protobuff structs
- Add build system that generates keys for circuits and caches them.


[#307]: https://github.com/dusk-network/rusk/issues/307
[#292]: https://github.com/dusk-network/rusk/issues/292
[#290]: https://github.com/dusk-network/rusk/issues/290
[0.1.0]: https://github.com/dusk-network/rusk/releases/tag/rusk-0.1.0
