# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Update to edition 2024
- Update `cargo_toml` dependency to 0.22
- Update MSRV to 1.85

### Fixed

- Fix clippy `io_other_error` warning

## [1.4.1] - 2026-02-11

### Added

- Add export of `dusk_vm::Session`

### Removed

- Remove `DEFAULT_SNAPSHOT` const

## [1.4.0] - 2025-11-06

### Removed

- Remove feature flag to use now stable `lazy_cell` feature

## [1.3.0] - 2025-04-17

### Changed

- Upgrade piecrust to `0.28.1`
- Upgrade `dusk-vm` to `1.3.0`
- Upgrade `dusk-core` to `1.3.0`

## [1.0.3] - 2025-02-14

### Changed

- Change deprecated `tempdir` with `tempfile` dependency [#3407]

## [1.0.2] - 2025-01-23

### Changed

- Change dependency declaration to not require strict equal [#3405]

## [1.0.1] - 2025-01-20

- First `rusk-recovery` release

<!-- Issues -->
[#3407]: https://github.com/dusk-network/rusk/issues/3407
[#3405]: https://github.com/dusk-network/rusk/issues/3405

[Unreleased]: https://github.com/dusk-network/rusk/compare/rusk-recovery-1.4.1...HEAD
[1.4.0]: https://github.com/dusk-network/rusk/compare/rusk-recovery-1.4.0...rusk-recovery-1.4.1
[1.4.0]: https://github.com/dusk-network/rusk/compare/rusk-recovery-1.3.0...rusk-recovery-1.4.0
[1.3.0]: https://github.com/dusk-network/rusk/compare/rusk-recovery-1.0.3...rusk-recovery-1.3.0
[1.0.3]: https://github.com/dusk-network/rusk/compare/rusk-recovery-1.0.2...rusk-recovery-1.0.3
[1.0.2]: https://github.com/dusk-network/rusk/compare/rusk-recovery-1.0.1...rusk-recovery-1.0.2
[1.0.1]: https://github.com/dusk-network/rusk/tree/rusk-recovery-1.0.1
