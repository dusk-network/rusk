<div align="center">

# `📨 Rusk Prover`

> Local PLONK zero-knowledge prover for Phoenix (shielded) transactions
</div>

## Overview

Rusk Prover generates PLONK zero-knowledge proofs for Phoenix transactions locally. It implements the `dusk_core::transfer::phoenix::Prove` trait and supports transaction circuits with 1 to 4 input notes (each producing 2 output notes). Prover keys are loaded lazily from [`rusk-profile`](../rusk-profile/).

## How It Works

1. Receives a serialized `TxCircuitVec` containing the transaction circuit data
2. Selects the matching circuit variant based on the number of input notes:
   - `1-in / 2-out`
   - `2-in / 2-out`
   - `3-in / 2-out`
   - `4-in / 2-out`
3. Loads the corresponding prover key from the profile directory (cached after first load)
4. Generates and returns the PLONK proof

## Features

| Feature | Description |
|---------|-------------|
| `unsafe_deterministic_rng` | **Unsafe for production**. Use a fixed seeded RNG for deterministic proofs (testing only) |
| `debug` | Enable tracing and hex logging of proof data |

## Related Crates

- [`dusk-core`](../core/) — defines the `Prove` trait and Phoenix circuit types
- [`rusk-profile`](../rusk-profile/) — stores and retrieves circuit prover keys
- [`rusk`](../rusk/) — uses the prover in prover node mode (`--features prover`)
- [`rusk-test`](../rusk-test/) — uses the prover with `unsafe_deterministic_rng` + `debug` for deterministic test proofs
