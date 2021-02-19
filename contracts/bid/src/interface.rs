// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Host interface for the Bid Contract.
//!
//! Here the interface of the contract that will be used to execute
//! functions of it from the host envoirnoment (Rust) is defined here.
//!
//! It mostly contains the function signatures that need to be exported
//! to the outside world (AKA outside WASM).

use crate::{ops, Contract};
use alloc::vec::Vec;
use canonical_host::MemStore;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::{PublicKey, StealthAddress};
use dusk_poseidon::cipher::PoseidonCipher;
use phoenix_core::Note;
use schnorr::Signature;

type TransactionIndex = u16;

impl Contract<MemStore> {
    /// This function allows to the contract caller to setup a Bid related to a
    /// one-time identity of his/her property that will allow the user to
    /// participate in the bidding process of the consensus as well as to
    /// prove that is part of the bidders set.
    ///
    /// This function will first of all, verify that the Bid is correct by
    /// verifying the BidCorrectness Proof.
    /// Then it will include the Bid into the PoseidonTree of the contract and
    /// link the One-Time identity of the user to the index that the bid
    /// occupies in the tree. Finally it will execute an inter-contract call
    /// sending the `spending_proof` and a `note` to the transfer contract.
    /// Which will execute the transaction of Dusk to the contract account.
    pub fn bid(
        commitment: JubJubAffine,
        hashed_secret: BlsScalar,
        nonce: BlsScalar,
        encrypted_data: PoseidonCipher,
        stealth_address: StealthAddress,
        block_height: u64,
        correctness_proof: Vec<u8>,
        spending_proof: Vec<u8>,
    ) -> Transaction<
        (
            TransactionIndex,
            JubJubAffine,
            BlsScalar,
            BlsScalar,
            PoseidonCipher,
            StealthAddress,
            u64,
            Vec<u8>,
            Vec<u8>,
        ),
        (bool, u64),
    > {
        Transaction::new((
            ops::BID,
            commitment,
            hashed_secret,
            nonce,
            encrypted_data,
            stealth_address,
            block_height,
            correctness_proof,
            spending_proof,
        ))
    }

    /// This function allows to the contract caller to withdraw it's `Bid` and
    /// therefore the funds placed to place it in the contract.
    ///
    /// Note that to be able to withdraw a `Bid`, it needs to reach a certain
    /// time which corresponds to the `expiration` time of the bid plus the
    /// `COOLDOWN_PERIOD`.
    ///
    /// Once this execution suceeds, any links between the bidder, as well as
    /// it's one-time identity and the Bid itself will be erased from the
    /// contract storage which will return some gas to the caller.
    pub fn withdraw(
        signature: Signature,
        pub_key: PublicKey,
        note: Note,
        spending_proof: Vec<u8>,
        block_height: u64,
    ) -> Transaction<
        (TransactionIndex, Signature, PublicKey, Note, Vec<u8>, u64),
        bool,
    > {
        Transaction::new((
            ops::WITHDRAW,
            signature,
            pub_key,
            note,
            spending_proof,
            block_height,
        ))
    }

    /// This function allows to the contract caller to extend the expiration
    /// time for his/her `Bid`. That means, remain longer in the Bidding
    /// consensus process with the same `Bid` and therefore the same
    /// One-time identity.
    pub fn extend_bid(
        signature: Signature,
        pub_key: PublicKey,
    ) -> Transaction<(TransactionIndex, Signature, PublicKey), bool> {
        Transaction::new((ops::EXTEND_BID, signature, pub_key))
    }
}
