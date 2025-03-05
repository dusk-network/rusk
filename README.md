<h1 align="center">
<img height="90" src="assets/rusk_logo_light.svg#gh-dark-mode-only" alt="Rusk">
<img height="90" src="assets/rusk_logo_dark.svg#gh-light-mode-only" alt="Rusk">
</h1>

<p align="center">
  The official <img height="11" src="assets/dusk_circular_light.svg#gh-dark-mode-only"><img height="11" src="assets/dusk_circular_dark.svg#gh-light-mode-only"><a href="https://dusk.network/"> Dusk</a> protocol node client and smart contract platform.
</p>

<p align=center>
<a href="https://github.com/dusk-network/rusk/actions/workflows/rusk_ci.yml">
<img src="https://github.com/dusk-network/rusk/actions/workflows/rusk_ci.yml/badge.svg" alt="Rusk CI"></a>
&nbsp;
<a href="https://github.com/dusk-network/rusk/actions/workflows/explorer_ci.yml">
<img src="https://github.com/dusk-network/rusk/actions/workflows/explorer_ci.yml/badge.svg" alt="explorer CI"></a>
&nbsp;
<a href="https://github.com/dusk-network/rusk/actions/workflows/webwallet_ci.yml">
<img src="https://github.com/dusk-network/rusk/actions/workflows/webwallet_ci.yml/badge.svg" alt="web-wallet CI"></a>
&nbsp;
<a href="">
<img alt="GitHub Repo stars" src="https://img.shields.io/github/stars/dusk-network/rusk?style=social"></a>
&nbsp;
<a href="https://discord.gg/dusk-official">
<img src="https://img.shields.io/discord/847466263064346624?label=discord&style=flat-square&color=5a66f6" alt="Discord"></a>
&nbsp;
<a href="https://x.com/DuskFoundation/">
<img alt="X (formerly Twitter) Follow" src="https://img.shields.io/twitter/follow/DuskFoundation"></a>
&nbsp;
<a href="https://docs.dusk.network">
<img alt="Static Badge" src="https://img.shields.io/badge/read%20the%20docs-E2DFE9?style=flat-square&logo=data%3Aimage%2Fsvg%2Bxml%3Bbase64%2CPHN2ZyB3aWR0aD0iMjAwIiBoZWlnaHQ9IjIwMCIgdmlld0JveD0iMCAwIDIwMCAyMDAiIGZpbGw9Im5vbmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI%2BCjxwYXRoIGZpbGwtcnVsZT0iZXZlbm9kZCIgY2xpcC1ydWxlPSJldmVub2RkIiBkPSJNODEuMjk4IDEuNzM3OEM4OC4yMjI5IDAuNDM3NzI0IDk1LjQyMjcgLTAuMTYyMzEyIDEwMi43OTggMC4wMzc3QzE1NC45OTYgMS40Mzc3OSAxOTcuODQ1IDQzLjc0MDQgMTk5LjkyIDk1LjkxODZDMjAyLjE3IDE1Mi45OTcgMTU2LjU3MSAyMDAgOTkuOTk3NiAyMDBDOTMuNjIyNyAyMDAgODcuMzcyOSAxOTkuNCA4MS4zMjMgMTk4LjI1QzM1LjAyNDIgMTg5LjQ5OSAwIDE0OC44MjIgMCA5OS45OTM5QzAgNTEuMTY1OCAzNC45OTkyIDEwLjQ4ODMgODEuMjk4IDEuNzM3OFpNMTAyLjc3MyAxNzYuNjc0QzEwMS43MjMgMTc4LjAyNCAxMDIuODIyIDE3OS45NzQgMTA0LjUyMiAxNzkuODc0QzE0Ni42MjEgMTc3LjUyNCAxNzkuOTk2IDE0Mi42NzEgMTc5Ljk5NiA5OS45OTM5QzE3OS45OTYgNTcuMzE2MiAxNDYuNTk2IDIyLjQ2NDEgMTA0LjQ5NyAyMC4xMTM5QzEwMi43OTggMjAuMDEzOSAxMDEuNzIzIDIxLjk2NDEgMTAyLjc3MyAyMy4zMTQxQzExOS4yNDcgNDQuNDY1NCAxMjkuMDQ3IDcxLjA5MjEgMTI5LjA0NyA5OS45OTM5QzEyOS4wNDcgMTI4Ljg5NiAxMTkuMjIyIDE1NS40OTcgMTAyLjc3MyAxNzYuNjc0WiIgZmlsbD0iIzEwMTAxMCIvPgo8L3N2Zz4K"></a>
</a>
</p>

> _Unstable_ : No guarantees can be made regarding the API stability, the
> project is in development.

# 🖧 How to run a node

This README is for people who want to develop, test nodes locally, and
contribute to the Rusk codebase.

For more information on **running a node for main- or testnet**, see our
[Node operator docs](https://docs.dusk.network/operator/overview/)

# 📃 Table of Contents

- [Prerequisites](#prerequisites)
  - [Setup script](#setup-script)
  - [Rust Installation](#rust-installation)
- [Build and Tests](#️-build-and-tests)
- [Run a local node for development](#-run-a-local-node-for-development)
  - [Spin up local node](#spin-up-local-node)
    - [Prepare modules](#prepare-modules)
    - [Run a node](#run-a-node)
    - [Run an archive node](#run-an-archive-node)
    - [Run a prover node](#run-a-prover-node)
- [Contracts compilation](#-contracts-compilation)
- [Docker support](#-docker-support)

## 📝 Prerequisites

- Rust 1.71 nightly or higher
- GCC 13 or higher
- Clang 16 or higher

### Setup script

We provide a setup script in the `scripts` folder that can take care of
everything.

```bash
bash scripts/dev-setup.sh
```

### Rust Installation

Rusk makes use of the nightly toolchain, make sure it is installed. Furthermore,
to build the WASM contracts, `wasm-pack` is required.

To install and set the nightly toolchain, and install `wasm-pack`, run:

```bash
rustup toolchain install nightly
rustup default nightly
cargo install wasm-pack
```

## 🛠️ Build and Tests

To build `rusk` from source, make sure the prerequisites are met. Then you can
simply run the following command to compile everything:

```bash
make
```

To run tests:

```bash
make test
```

That will also compile all the genesis contracts and its associated circuits.
See also `make help` for all the available commands

## 💻 Run a local node for development

### Spin up local node

Run a single full-node cluster with example state.

#### Prepare modules:

```bash
# Generate the keys used by the circuits
# Compile all the genesis contracts
# Copy example consensus.keys
make prepare-dev
```

#### Run a Node

```bash
# Launch a local ephemeral node
make run-dev
```

#### Run an Archive node

```bash
make run-dev-archive
```

#### Run a Prover Node

The node can be build as a prover only as follows:

```bash
cargo r --release --no-default-features --features prover -p dusk-rusk
```

This prover node will be accessible on `https://localhost:8080`. Apps like the
[rusk-wallet](https://github.com/dusk-network/rusk/tree/master/rusk-wallet) can
be connected to it for quicker and more private local proving.

## 📜 Contracts compilation

Compile all the genesis contracts without running the server:

```bash
make contracts
```

Compile a specific genesis contract:

```bash
# generate the wasm for `transfer` contract
make wasm for=transfer
```

## 🐳 Docker support

It's also possible to run a local ephemeral node with Docker.

To build the Docker image with archive:

```bash
docker build -t rusk .
```

To build the Docker image **without** archive:

```bash
docker build -t rusk --build-arg CARGO_FEATURES="" .
```

To run Rusk inside a Docker container:

```bash
docker run -p 9000:9000/udp -p 8080:8080/tcp rusk
```

Port 9000 is used for Kadcast, port 8080 for the HTTP and GraphQL APIs.

## License

The Rusk software is licensed under the
[Mozilla Public License Version 2.0](./LICENSE).
