<div align="center">

# `🔗 Dusk Consensus`

> Implementation of Dusk's [Succinct Attestation](https://github.com/dusk-network/docs/blob/main/src/content/docs/learn/deep-dive/succinct-attestation.md) consensus protocol
</div>

## Overview

Dusk Consensus implements the Succinct Attestation (SA) protocol, which drives block production and finality on the Dusk network. It coordinates a multi-phase process — proposal, validation, and ratification — where provisioners vote on candidate blocks and aggregate their votes into a compact attestation.

## Key Components

| Component | Description |
|-----------|-------------|
| Consensus state machine | Drives the proposal / validation / ratification phases |
| Vote aggregation | Collects and verifies BLS-signed votes from provisioners |
| Quorum logic | Determines when sufficient stake weight has voted to finalize |
| Merkle aggregation | Batches proofs for efficient on-chain verification |

## Related Crates

- [`node-data`](../node-data/) — defines consensus message types and ledger structures
- [`dusk-core`](../core/) — BLS signatures used for vote signing
- [`node`](../node/) — integrates the consensus engine into the full node runtime
- [`rusk`](../rusk/) — orchestrates consensus as part of the node entrypoint
