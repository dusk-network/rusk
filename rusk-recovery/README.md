<div align="center">

# `⬇️ Rusk Recovery`

> Bootstrap and recovery utilities for initializing a Dusk node
</div>

## Overview

Rusk Recovery provides tools to bootstrap a new node or restore an existing one. It can deploy genesis state from snapshot configurations and download circuit proving keys, writing everything into the [`rusk-profile`](../rusk-profile/) directory.

## Features

Both features are optional and gated behind Cargo feature flags:

| Feature | Description |
|---------|-------------|
| `state` | Deploy genesis state from snapshots (local TOML, HTTP, zip, or tar archives) |
| `keys` | Download and verify ZK circuit proving keys from the network |

### State Recovery

The `state` feature provides:
- `Snapshot` — genesis configuration defining initial balances and stakes
- `PhoenixBalance` — shielded (Phoenix) balance entries
- `GenesisStake` — initial stake allocations
- `deploy()` — populates the transfer and stake contracts with genesis data

### Key Recovery

The `keys` feature downloads circuit prover/verifier keys and verifies their integrity before storing them in the profile directory.

## Usage

Recovery is typically invoked through Make targets rather than directly:

```bash
make keys          # Download circuit proving keys
make state         # Generate genesis state
make prepare-dev   # Keys + state + example consensus keys
```

## Related Crates

- [`rusk-profile`](../rusk-profile/) — target directory for recovered state and keys
- [`dusk-vm`](../vm/) — used to deploy genesis state into the VM
- [`rusk`](../rusk/) — invokes recovery during first-run initialization
- [`rusk-test`](../rusk-test/) — uses state recovery to set up ephemeral test nodes
