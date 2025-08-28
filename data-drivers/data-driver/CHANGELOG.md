# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2025-08-28

### Added

- Add `alloc` feature
- Add standard wasm implementation

### Removed

- Remove wasm-bindgen dependency

## [0.1.0] - 2025-04-17

### Added

- Add `ConvertibleContract` trait for seamless conversion between JSON and RKYV formats.
- Add `rkyv_to_json` and `json_to_rkyv` functions for bidirectional serialization.
- Add support for encoding and decoding function inputs, outputs, and events in RKYV.

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-data-driver-0.2.0...HEAD
[0.2.0]: https://github.com/dusk-network/rusk/compare/dusk-data-driver-0.1.0...dusk-data-driver-0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/dusk-data-driver-0.1.0
