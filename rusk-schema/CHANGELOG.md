# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add `GetNotes` call to stream notes from the server [#702]

### Deprecated

- Mark `GetNotesOwnedByRequest` fields as `[deprecated=true]` [#702]

## [0.3.0] - 2021-04-26

### Added

- Add `discarded_txs` field to `ExecuteStateTransitionResponse` [#704]

### Removed

- Remove `success` field from `ExecuteStateTransitionResponse` [#704]

## [0.2.0] - 2021-04-15

### Added

- Add the latest block `height` in `GetNotesOwnedByResponse` [#651]
- Insert `generator` in state transition requests [#699]

### Changed

- Change `Stake` and `GetStakeResponse` to support new stake contract spec [#614]

## [0.1.0] - 2021-04-05

### Added

- Initial release

[#704]: https://github.com/dusk-network/rusk/issues/704
[#702]: https://github.com/dusk-network/rusk/issues/702
[#699]: https://github.com/dusk-network/rusk/issues/699
[#651]: https://github.com/dusk-network/rusk/issues/651
[#614]: https://github.com/dusk-network/rusk/issues/614

<!-- Releases -->

[Unreleased]: https://github.com/dusk-network/rusk/compare/rusk-schema-v0.3.0...HEAD
[0.3.0]: https://github.com/dusk-network/rusk/releases/tag/rusk-schema-v0.2.0...rusk-schema-v0.3.0
[0.2.0]: https://github.com/dusk-network/rusk/releases/tag/rusk-schema-v0.1.0...rusk-schema-v0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/releases/tag/rusk-schema-v0.1.0
