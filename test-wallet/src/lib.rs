// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The wallet specification.

#![deny(missing_docs)]
#![deny(clippy::all)]
#![allow(clippy::result_large_err)]

extern crate alloc;

mod imp;

use alloc::vec::Vec;
use poseidon_merkle::Opening as PoseidonOpening;
use zeroize::Zeroize;

use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    stake::StakeData,
    transfer::{
        moonlight::AccountData,
        phoenix::{
            Note, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
            Transaction as PhoenixTransaction, ViewKey as PhoenixViewKey,
            NOTES_TREE_DEPTH,
        },
        Transaction,
    },
    BlsScalar,
};

pub use wallet_core::keys::{
    derive_bls_sk, derive_phoenix_pk, derive_phoenix_sk,
};

pub use imp::*;

/// Types that are client of the prover.
pub trait ProverClient {
    /// Error returned by the node client.
    type Error;

    /// Requests that a node prove the given unproven [`PhoenixTransaction`] and
    /// later propagates it.
    ///
    /// # Errors
    /// This function will error if the proof can not be generated from the
    /// unproven transaction.
    fn compute_proof_and_propagate(
        utx: &PhoenixTransaction,
    ) -> Result<Transaction, Self::Error>;
}

/// Stores the cryptographic material necessary to derive cryptographic keys.
pub trait Store {
    /// The error type returned from the store.
    type Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error>;

    /// Retrieve the secret key with the given index.
    fn phoenix_secret_key(
        &self,
        index: u8,
    ) -> Result<PhoenixSecretKey, Self::Error> {
        let mut seed = self.get_seed()?;

        let sk = derive_phoenix_sk(&seed, index);

        seed.zeroize();

        Ok(sk)
    }

    /// Retrieve the public key with the given index.
    fn phoenix_public_key(
        &self,
        index: u8,
    ) -> Result<PhoenixPublicKey, Self::Error> {
        let mut seed = self.get_seed()?;

        let pk = derive_phoenix_pk(&seed, index);

        seed.zeroize();

        Ok(pk)
    }

    /// Retrieve the account secret key with the given index.
    fn account_secret_key(
        &self,
        index: u8,
    ) -> Result<BlsSecretKey, Self::Error> {
        let mut seed = self.get_seed()?;

        let sk = derive_bls_sk(&seed, index);

        seed.zeroize();

        Ok(sk)
    }

    /// Retrieve the account public key with the given index.
    fn account_public_key(
        &self,
        index: u8,
    ) -> Result<BlsPublicKey, Self::Error> {
        let mut seed = self.get_seed()?;

        let mut sk = derive_bls_sk(&seed, index);
        let pk = BlsPublicKey::from(&sk);

        seed.zeroize();
        sk.zeroize();

        Ok(pk)
    }
}

/// Tuple containing Note and block height
pub type EnrichedNote = (Note, u64);

/// Types that are clients of the state API.
pub trait StateClient {
    /// Error returned by the node client.
    type Error;

    /// Find notes for a view key.
    fn fetch_notes(
        &self,
        vk: &PhoenixViewKey,
    ) -> Result<Vec<EnrichedNote>, Self::Error>;

    /// Fetch the current root of the state.
    fn fetch_root(&self) -> Result<BlsScalar, Self::Error>;

    /// Asks the node to return the nullifiers that already exist from the given
    /// nullifiers.
    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error>;

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonOpening<(), NOTES_TREE_DEPTH>, Self::Error>;

    /// Queries the node for the stake of a key. If the key has no stake, a
    /// `Default` stake info should be returned.
    fn fetch_stake(&self, pk: &BlsPublicKey) -> Result<StakeData, Self::Error>;

    /// Queries the account data for a given key.
    fn fetch_account(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, Self::Error>;
}
