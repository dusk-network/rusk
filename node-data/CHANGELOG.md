# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add zeroize for secret info [#3687]
- Add serde derive for Block Label

### Changed

- Change provisioner keys encryption [#3391]

## [1.3.0] - 2025-04-17

### Changed

- Make Transaction `size` method infallible
- Make Header `size` method infallible

## [1.2.0] - 2025-03-20

### Removed

- Remove `WrappedContractId` [#3503]

## [1.1.0] - 2025-02-14

### Added

- Add PartialEq, Eq to `BlockState` [#3359]
- Add `SpentTransaction::shielded` & `SpentTransaction::public` getter fn [#3464]

### Removed

- Removed `ArchivalData` together with archive module [#3359]

[1.0.1] - 2025-01-23

### Changed

- Change dependency declaration to not require strict equal [#3405]

## [1.0.0] - 2025-01-16

### Added

- Add Types used for interacting with Dusk node 

<!-- Issues -->
[#3687]: https://github.com/dusk-network/rusk/issues/3687
[#3503]: https://github.com/dusk-network/rusk/issues/3503
[#3464]: https://github.com/dusk-network/rusk/issues/3464
[#3391]: https://github.com/dusk-network/rusk/issues/3391
[#3359]: https://github.com/dusk-network/rusk/issues/3359
[#3405]: https://github.com/dusk-network/rusk/issues/3405

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-node-data-1.3.0...HEAD
[1.3.0]: https://github.com/dusk-network/rusk/compare/dusk-node-data-1.2.0...dusk-node-data-1.3.0
[1.2.0]: https://github.com/dusk-network/rusk/compare/dusk-node-data-1.1.0...dusk-node-data-1.2.0
[1.1.0]: https://github.com/dusk-network/rusk/compare/dusk-node-data-1.0.1...dusk-node-data-1.1.0
[1.0.1]: https://github.com/dusk-network/rusk/compare/dusk-node-data-1.0.0...dusk-node-data-1.0.1
[1.0.0]: https://github.com/dusk-network/rusk/tree/dusk-node-data-1.0.0
