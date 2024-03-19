# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

- Keep last epoch state commit as a reversion target [#1094]
- Allow state transitions to be executed in parallel with queries [#970]
- Change dependencies declarations enforce bytecheck [#1371]
- Fixed tests passing incorrect arguments [#1371]

### Added

- Add type constrains for bytecheck [#1371]
- Add TLS support for HTTP server
- Add iteration generator to FailedIterations [#1257]
- Add `node` feature flag [#1144]

### Changed

- Change rusk::provisioners to filter out slashed stakes [#1257]
- Change block processing to slash failed-iteration provisioners [#1257]
- Change FailedIterations to include only nil quorum certificates [#1257]

### Removed

- Remove allowlist [#1257]

## [0.7.0] - 2023-12-31

### Added

- Add gas price API: max, min and mean gas prices
- Add rolling finality
- Add AST benchmarks
- Add common module to benchmark
- Add criterion dependency for benchmarks
- Add `prover` feature flag as default
- Add hash header to the CRS-via-HTTP api
- Add support for HTTP response headers
- Add test for unspendable transaction
- Add HTTP `crs` topic
- Add HTTP support for explicit binary response
- Add `rusk-recovery` subcommands
- Add `info` HTTP endpoint
- Add HTTP support for `application/json` content type
- Add HTTP version handshake
- Add `network_id` config
- Add `build` recipe to Makefile
- Add `--profile` arg
- Add GQL mempool API
- Add `alive_nodes` api
- Add `provisioners` api
- Add GQL block state hash field
- Add blocks by range graphql handler
- Add transactions by block range graphql handler
- Add tests for HTTP handler
- Add `HandleRequest` trait
- Add `propagate_tx` http handler
- Add `prove_*` http handler
- Add `preverify` http handler
- Add `MessageRequest::to_error`
- Add http format request
- Add support for gql variables
- Add various gql queries
- Add support for binary http requests
- Add handler for query_raw
- Add basic graphql implementation
- Add `multi_transfer` test
- Add `gas_behaviour` test
- Add `stake` test
- Add `wallet` test (aka `wallet_grpc`)
- Add `get_notes` functionality
- Add `preverify` implementation of node's trait
- Add client request parsing
- Add default for WsConfig
- Add listening for incoming websocket streams
- Add node dependency

### Changed

- Change `dusk` key
- Change `finalize` to not remove previous vm commit
- Change `Provisioners::default()` to `Provisioners::empty()`
- Change http `prover` to activate behind feature flag
- Change `clippy` to cover all features
- Change `stake.toml` configuration for tests
- Change error messages while accepting blocks
- Change VST to use candidate gas_limit
- Change BlockGenerator to produce the correct gas_limit value
- Change gql dep to `async-graphql`
- Change external event system implementation
- Change `consistency_check` to `Option`
- Change codebase to support new `SpentTransaction`

### Removed

- Remove obsoleted test
- Remove multiple stakes from provisioner member
- Remove timing for subcommands
- Remove `RUSK_PROFILE_PATH` dependency
- Remove `rusk-recovery` as library dependency.
- Remove tracing prefix
- Remove unnecessary quotes from text response
- Remove `Ws` prefix for `http` module structs
- Remove manual async executor instantiation
- Remove/disable deprecated tests
- Remove dependencies from binary
- Remove obsolete dependencies
- Remove `rusk-schema` dependency
- Remove grpc server

### Fixed

- Fix binary HTTP event detection
- Fix HTTP header parsing for unquoted strings
- Fix HTTP header parsing for empty values
- Fix default HTTP_Listener
- Fix graphql latest transaction filter
- Fix http listener activation
- Fix double execution for incoming request
- Fix gas spent for ICC
- Fix json response serialization
- Fix Event::parse
- Fix routing for Host("rusk")

## [0.6.0] - 2023-06-21

## [0.5.3] - 2022-03-21

## [0.4.0] - 2021-08-13

## [0.3.0] - 2021-07-15

### Added
- Prover service implementation [#410]

### Changed

- Update `dusk-poseidon` to `0.22.0-rc` [#327]
- Update `dusk-pki` to `0.8.0-rc` [#327]
- Update `phoenix-core` to `0.13.0-rc` [#327]
- Update `dusk-schnorr` to `0.8.0-rc` [#327]
- Update the blindbid service to use the new API of `dusk-blindbid` [#327]

### Removed

- Remove bid contracts and circuits [#369]
- Remove `dusk-blindbid` crate and rusk's blindbid services [#369]

## [0.2.0] - 2021-05-19

### Added

- Add setup/teardown system for tests [#292]
- Add `dusk-jubjub v0.10` to deps [#292]
- Add `async-stream` to deps [#292]
- Add `test-context v0.1` to dev-dependencies [#292]
- Add `async-trait v0.1` to dev-dependencies [#292]
- Add `RUSK_PROFILE_PATH` env variable check in `build.rs` [#307]

### Changed

- Update build system improving Circuit keys caching [#290]
- Update `tokio` to `v1.6` [#292]
- Update `tonic` from `0.3` to `0.4` [#292]
- Update `prost` from `0.6` to `0.7` [#292]
- Change `tower` to be a dev-dependency [#292]
- Refactor `unix` modules from `tests` and `bin` [#292]

### Fixed

- Fix dusk-bytes encoding issues [#292]
- Fix score generation module/service [#292]

### Removed

- Remove `bincode` since is unused [#292]
- Remove `default-feats = false` from `dusk-plonk` [#292]
- Remove `thiserror` from dependencies [#292]

## [0.1.0] - 2021-02-19

### Added

- Add Rusk server impl
- Add BlindBid service
- Add Pki service
- Add Echo service
- Add encoding module
- Add clap cli interface
- Add linking between Rusk and Protobuff structs
- Add build system that generates keys for circuits and caches them.

[#1371]: https://github.com/dusk-network/rusk/issues/1371
[#1257]: https://github.com/dusk-network/rusk/pull/1257
[#1219]: https://github.com/dusk-network/rusk/issues/1219
[#1144]: https://github.com/dusk-network/rusk/issues/1144
[#1094]: https://github.com/dusk-network/rusk/issues/1094
[#970]: https://github.com/dusk-network/rusk/issues/970
[#401]: https://github.com/dusk-network/rusk/issues/401
[#369]: https://github.com/dusk-network/rusk/issues/369
[#327]: https://github.com/dusk-network/rusk/issues/327
[#307]: https://github.com/dusk-network/rusk/issues/307
[#292]: https://github.com/dusk-network/rusk/issues/292
[#290]: https://github.com/dusk-network/rusk/issues/290


[unreleased]: https://github.com/dusk-network/rusk/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/dusk-network/rusk/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/dusk-network/rusk/compare/v0.5.3...v0.6.0
[0.5.3]: https://github.com/dusk-network/rusk/compare/v0.4.0...v0.5.3
[0.4.0]: https://github.com/dusk-network/rusk/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/dusk-network/rusk/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/dusk-network/rusk/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/releases/tag/rusk-0.1.0
