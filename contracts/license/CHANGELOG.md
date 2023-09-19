# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Added method 'get_info' [#1052]

### Changed

- Changed 'use_license' to check if license already nullified [#1051]
- Changed 'get_licenses' to return values by adding 'pos' to every license returned [#1040]
- Changed 'issue_license' by removing the 'pos' argument and self position determination [#1039]

## [0.1.0] - 2023-07-13

### Added

- Add `license` contract to Rusk [#960]

[#1052]: https://github.com/dusk-network/rusk/issues/1052
[#1051]: https://github.com/dusk-network/rusk/issues/1051
[#1040]: https://github.com/dusk-network/rusk/issues/1040
[#1039]: https://github.com/dusk-network/rusk/issues/1039
[#960]: https://github.com/dusk-network/rusk/issues/960
