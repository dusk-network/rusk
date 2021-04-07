# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added
- Add `payment_info` host function [#254]

### Changed
- Change `verify_proof` to accept verifier data [#247]


## [0.2.0] - 2021-03-12

### Added

- Add `verify_proof` host function [#227]
- Add `PublicInput` enum wrapper for input types
- Add `PublicParameters` as field of `RuskModule`
- Add Schnorr Signature verification host function

### Changed

- Change Build Status shield URL

### Removed

- Remove clippy warnings

## [0.1.0] - 2021-02-19

### Added

- Add ABI infrastracture
- Add Poseidon Hash host function
- Add test contract
- Add CHANGELOG.md
- Add LICENSE
- Add README.md

[#227]: https://github.com/dusk-network/rusk/issues/227
[#254]: https://github.com/dusk-network/rusk/issues/254
[0.1.0]: https://github.com/dusk-network/dusk-abi/releases/tag/v0.1.0
