# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Change `lru` dependency to `0.16.3`

## [1.4.3] - 2026-01-07

### Fixed

- Fix host queries `verify_plonk`, `verify_groth16_bn254` and `verify_bls_multisig` to never panic

## [1.4.2] - 2025-12-13

### Added

- Add `remove_3rd_party` api
- Add `recompile_3rd_party` api

### Changed

- Change piecrust version requirement to 0.29.0-rc.3

## [1.4.1] - 2025-12-04

### Added

- Add range support for feature activation

## [1.4.0] - 2025-11-06

### Added

- Add support for `TransactionData::Blob`
- Add `keccak256` host query function [#3774]
- Add activaction height for host queries

## [1.3.0] - 2025-04-17

### Changed

- Change piecrust version requirement to `0.28.1`

## [1.2.0] - 2025-03-20

### Changed

- Change piecrust version requirement to `0.28.0`

## [1.1.0] - 2025-02-14

### Added

- Add `PUBLIC_SENDER` available to session [#3341]

### Changed

- Change `execution` module to use `execution::Config` [#3437]
- Change `dusk-core` dependency to `1.0.1-alpha` [#3341]
- Change `piecrust` dependency to `0.27.1` [#3341]

## [1.0.0] - 2025-01-23

### Changed

- Change dependency declaration to not require strict equal [#3405]

## [0.1.0] - 2025-01-14

### Added

- Add vm to interact with Dusk network [#3235]

<!-- Issues -->
[#3774]: https://github.com/dusk-network/rusk/issues/3774
[#3235]: https://github.com/dusk-network/rusk/issues/3235
[#3341]: https://github.com/dusk-network/rusk/issues/3341
[#3405]: https://github.com/dusk-network/rusk/issues/3405
[#3437]: https://github.com/dusk-network/rusk/issues/3437

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.4.3...HEAD
[1.4.3]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.4.2...dusk-vm-1.4.3
[1.4.2]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.4.1...dusk-vm-1.4.2
[1.4.1]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.4.0...dusk-vm-1.4.1
[1.4.0]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.3.0...dusk-vm-1.4.0
[1.3.0]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.2.0...dusk-vm-1.3.0
[1.2.0]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.1.0...dusk-vm-1.2.0
[1.1.0]: https://github.com/dusk-network/rusk/compare/dusk-vm-1.0.0...dusk-vm-1.1.0
[1.0.0]: https://github.com/dusk-network/rusk/compare/dusk-vm-0.1.0...dusk-vm-1.0.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/dusk-vm-0.1.0
