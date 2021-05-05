// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # bid-circuits
//! ![Build Status](https://github.com/dusk-network/rusk/workflows/Continuous%20integration/badge.svg)
//! [![Repository](https://img.shields.io/badge/github-bid--circuits-blueviolet?logo=github)](https://github.com/dusk-network/rusk/tree/master/circuits/bid)
//! [![Documentation](https://img.shields.io/badge/docs-bid--circuits-blue?logo=rust)](https://docs.rs/bid-circuits/)
//!
//! ## Contents
//!
//! This library provides the implementation of the [`BlindBidCircuit`] which allows the user
//! to construct and verify a Proof of BlindBid.
//!
//! Specifically, the circuit makes sure that:
//! 1. The public commitment is indeed the result of: `GENERATOR * bid_value + GENERATOR_NUMS * bid_blinder`.
//! 2. The bid_value relies in the range [50_000, 250_000].
//!
//! ### Example
//! ```rust
//! use rand::{Rng, thread_rng};
//! use dusk_plonk::prelude::*;
//! use dusk_plonk::jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
//! use bid_circuits::BidCorrectnessCircuit;
//!
//! // ------- Common perspective ------- //
//! // Generate Public Parameters (CRS)
//! let pub_params =
//!     PublicParameters::setup(1<<12, &mut thread_rng()).expect("CRS generation error");
//! // Generate ProverKey & VerifierData (ie. compile the circuit)
//! let (pk, vd) = BidCorrectnessCircuit::default().compile(&pub_params).expect("Compilation error");
//!
//! // ------- Prover perspective ------- //
//! // We're the owners of the bid and so we know the following
//! // fields:
//! let bid_value = JubJubScalar::from(60_000u64);
//! let bid_blinder = JubJubScalar::random(&mut thread_rng());
//! let commitment = JubJubAffine::from((GENERATOR_EXTENDED * bid_value) + (GENERATOR_NUMS_EXTENDED * bid_blinder));
//! let mut circuit = BidCorrectnessCircuit {
//!     commitment,
//!     value: bid_value,
//!     blinder: bid_blinder,
//! };
//! let proof = circuit.gen_proof(&pub_params, &pk, b"BidCorrectness").expect("Prove generation error");
//!
//! // ------- Verifier perspective ------- //
//! // Initialize your `PublicInputValue` vector.
//! let pi: Vec<PublicInputValue> = vec![
//!     commitment.into(),
//! ];
//!
//! assert!(circuit::verify_proof(&pub_params, &vd.key(), &proof, &pi, &vd.pi_pos(), b"BidCorrectness").is_ok())
//! ```
//!
//! ## Rationale & Theory
//!
//! In order to participate in the SBA consensus, Block generators have to
//! submit a bid in DUSK. As long as their bid is active - and their full-node
//! is connected with the internet and running- they are participating in the
//! consensus rounds. Essentially, every time a consensus round is run, the
//! Block Generator software generates a comprehensive zero-knowledge proof, and
//! executes various steps in order to generate a valid candidate block, and
//! compete with the other Block Generators for a chance to become the winner of
//! the consensus round.
//!
//! ![](https://user-images.githubusercontent.com/1636833/107039506-468c9e80-67be-11eb-9fb1-7ba999b3d6dc.png)
//!
//! Below we describe the three main processes that happen
//! every consensus round. Please note that 1 and 2 are run as part of the same
//! algorithm.
//!
//! ## Documentation
//! The best usage example of this library can actually be found in the rusk library.
//! This is the place where this lib provides all it's functionallities in order to check the correctness of the
//! Bids of the bidders.
//! See: <https://github.com/dusk-network/rusk/tree/master/rusk for more info and detail.>
//!
//! You can also check the documentation of this crate [here](https://docs.rs/bid-circuits/0.1.0/).
//!
//! ### Licensing
//! This code is licensed under Mozilla Public License Version 2.0 (MPL-2.0).
//! Please see [LICENSE](https://github.com/dusk-network/rusk/blob/master/circuits/bid) for further info.
//!
//! ### About
//! Protocol & Implementation designed by the [dusk](https://dusk.network) team.
//!
//! ### Contributing
//! - If you want to contribute to this repository/project please, check [CONTRIBUTING.md](https://github.com/dusk-network/rusk/blob/master/CONTRIBUTING.md)
//! - If you want to report a bug or request a new feature addition, please open
//!   an issue on this repository.

mod correctness;
pub use correctness::BidCorrectnessCircuit;
