# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fix invalid Quorum message with NoQuorum result [#3543]
- Fix `Quorum discarded` messages

### Changed

- Change `set_step_votes` to return an Attestation instead of a Quorum message
- Remove `build_quorum_msg` from `AttInfoRegistry`

## [1.2.0] - 2025-03-20

### Fixed

- Fix `MismatchHeight` error message

## [1.0.1] - 2025-01-23

## [1.0.0] - 2025-01-16

- First `dusk-consensus` release

<!-- Issues -->
[#3543]: https://github.com/dusk-network/rusk/issues/3543

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-consensus-1.2.0...HEAD
[1.2.0]: https://github.com/dusk-network/rusk/compare/dusk-consensus-1.0.1...dusk-consensus-1.2.0
[1.0.1]: https://github.com/dusk-network/rusk/compare/consensus-1.0.0...dusk-consensus-1.0.1
[0.1.0]: https://github.com/dusk-network/rusk/tree/consensus-1.0.0

