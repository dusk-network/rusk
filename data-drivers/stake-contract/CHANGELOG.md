# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.7.0] - 2026-06-10

### Fixed

- Add missing `decode_input_fn` support for `stakes` so query inputs are
  round-trip decodable.
- Add `prev_state_changes` feeder query mapping to schema, input decoding, and
  output decoding.

## [0.3.0] - 2025-11-06

### Changed

- Change `data-driver` dependency to `0.3.0`

## [0.1.0] - 2025-04-17

### Added

- Add implementation for `ConvertibleContract`

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-stake-contract-dd-1.7.0...HEAD
[1.7.0]: https://github.com/dusk-network/rusk/compare/dusk-stake-contract-dd-0.3.0...dusk-stake-contract-dd-1.7.0
[0.3.0]: https://github.com/dusk-network/rusk/compare/dusk-stake-contract-dd-0.1.0...dusk-stake-contract-dd-0.3.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/dusk-stake-contract-dd-0.1.0
