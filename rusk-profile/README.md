<div align="center">

# `🪪 Rusk Profile`

> Manages the local Rusk profile directory for circuit artifacts, proving keys, and genesis state
</div>

## Overview

Rusk Profile manages the `~/.dusk/rusk/` directory tree that stores ZK circuit artifacts, proving/verifier keys, the Common Reference String (CRS), consensus keys, and genesis state. It provides integrity verification (blake3, SHA-256) for all trusted setup artifacts.

## Directory Layout

```
~/.dusk/rusk/
├── circuits/    # ZK circuit prover/verifier keys (blake3-verified)
├── keys/        # Consensus keys
├── state/       # Genesis and recovered chain state
└── drivers/     # Data-driver WASM storage
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUSK_PROFILE_PATH` | Override the default `~/.dusk/rusk` profile location |

## Related Crates

- [`rusk-prover`](../rusk-prover/) — loads circuit prover keys from the profile
- [`rusk-recovery`](../rusk-recovery/) — writes state and keys into the profile
- [`rusk`](../rusk/) — reads genesis state from the profile at startup
