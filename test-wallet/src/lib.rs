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

use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_core::stake::StakeData;
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::phoenix::{
    Note, NoteOpening, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
};
use dusk_core::BlsScalar;
use zeroize::Zeroize;

pub use wallet_core::keys::{
    derive_bls_sk, derive_phoenix_pk, derive_phoenix_sk,
};

pub use imp::*;

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

/// Types that are clients of the state API.
pub trait StateClient {
    /// Error returned by the node client.
    type Error;

    /// Find notes for a view key.
    fn fetch_notes(
        &self,
        vk: &PhoenixViewKey,
    ) -> Result<Vec<(Note, u64)>, Self::Error>;

    /// Fetch the current root of the state.
    fn fetch_root(&self) -> Result<BlsScalar, Self::Error>;

    /// Asks the node to return the nullifiers that already exist from the given
    /// nullifiers.
    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error>;

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(&self, note: &Note) -> Result<NoteOpening, Self::Error>;

    /// Queries the node for the stake of a key. If the key has no stake, a
    /// `Default` stake info should be returned.
    fn fetch_stake(&self, pk: &BlsPublicKey) -> Result<StakeData, Self::Error>;

    /// Queries the account data for a given key.
    fn fetch_account(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, Self::Error>;

    /// Queries for the chain ID.
    fn fetch_chain_id(&self) -> Result<u8, Self::Error>;
}
