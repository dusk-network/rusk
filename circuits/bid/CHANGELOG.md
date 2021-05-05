# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added 
- Add code-hasher to define CIRCUIT_ID [#277]
- Add rusk-profile as CRS setup [#278]
- Add crate docs [#278]
- Add README.md [#278]

### Changed
- Update `dusk-plonk` from `v0.6` to `v0.8` [#277]
- Update `plonk_gadgets` from `v0.5` to `v0.6` [#277]
- Update `dusk-blindbid` from `v0.6` to `v0.8` [#277]
- Change `rand` as dev-dependency [#277]
- Rename `CorrectnessCircuit` to `BidCorrectnessCircuit` [#277]
- Refactor circuit to contain JubJubScalars instead of Bls ones [#277]

### Removed
- Remove `anyhow` usage [#277]

## [0.1.0] - 2021-02-16

### Added
- Add first `bid-circuits` implementation [#99]
- Add `bid-circuits` as workspace member [#207]

[#99]: https://github.com/dusk-network/rusk/issues/99
[#207]: https://github.com/dusk-network/rusk/issues/207
[#277]: https://github.com/dusk-network/rusk/issues/277
[#278]: https://github.com/dusk-network/rusk/issues/278
[0.1.0]: https://github.com/dusk-network/rusk/releases/tag/bid-circuits-0.1.0
