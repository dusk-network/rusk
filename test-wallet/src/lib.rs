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
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    stake::StakeData,
    transfer::{
        moonlight::AccountData,
        phoenix::{
            Note, SecretKey as PhoenixSecretKey, ViewKey, NOTES_TREE_DEPTH,
        },
        Transaction,
    },
    BlsScalar,
};
use poseidon_merkle::Opening as PoseidonOpening;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};

pub use imp::*;
pub use rusk_prover::UnprovenTransaction;

/// The maximum size of call data.
pub const MAX_CALL_SIZE: usize = execution_core::ARGBUF_LEN;

/// Stores the cryptographic material necessary to derive cryptographic keys.
pub trait Store {
    /// The error type returned from the store.
    type Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error>;

    /// Retrieves a derived secret key from the store.
    ///
    /// The provided implementation simply gets the seed and regenerates the key
    /// every time with [`generate_sk`]. It may be reimplemented to
    /// provide a cache for keys, or implement a different key generation
    /// algorithm.
    fn fetch_secret_key(
        &self,
        index: u64,
    ) -> Result<PhoenixSecretKey, Self::Error> {
        let seed = self.get_seed()?;
        Ok(derive_sk(&seed, index))
    }

    /// Retrieves a derived account secret key from the store.
    ///
    /// The provided implementation simply gets the seed and regenerates the key
    /// every time with [`generate_sk`]. It may be reimplemented to
    /// provide a cache for keys, or implement a different key generation
    /// algorithm.
    fn fetch_account_secret_key(
        &self,
        index: u64,
    ) -> Result<BlsSecretKey, Self::Error> {
        let seed = self.get_seed()?;
        Ok(derive_stake_sk(&seed, index))
    }
}

/// Generates a secret spend key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_sk(seed: &[u8; 64], index: u64) -> PhoenixSecretKey {
    let mut hash = Sha256::new();

    hash.update(seed);
    hash.update(index.to_le_bytes());
    hash.update(b"SSK");

    let hash = hash.finalize().into();
    let mut rng = ChaCha12Rng::from_seed(hash);

    PhoenixSecretKey::random(&mut rng)
}

/// Generates a secret key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_stake_sk(seed: &[u8; 64], index: u64) -> BlsSecretKey {
    let mut hash = Sha256::new();

    hash.update(seed);
    hash.update(index.to_le_bytes());
    hash.update(b"SK");

    let hash = hash.finalize().into();
    let mut rng = ChaCha12Rng::from_seed(hash);

    BlsSecretKey::random(&mut rng)
}

/// Types that are client of the prover.
pub trait ProverClient {
    /// Error returned by the node client.
    type Error;

    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Transaction, Self::Error>;
}

/// Block height representation
pub type BlockHeight = u64;

/// Tuple containing Note and Block height
pub type EnrichedNote = (Note, BlockHeight);

/// Types that are clients of the state API.
pub trait StateClient {
    /// Error returned by the node client.
    type Error;

    /// Find notes for a view key.
    fn fetch_notes(
        &self,
        vk: &ViewKey,
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

/// Information about the balance of a particular key.
#[derive(Debug, Default, Hash, Clone, Copy, PartialEq, Eq)]
pub struct BalanceInfo {
    /// The total value of the balance.
    pub value: u64,
    /// The maximum _spendable_ value in a single transaction. This is
    /// different from `value` since there is a maximum number of notes one can
    /// spend.
    pub spendable: u64,
}

impl Serializable<16> for BalanceInfo {
    type Error = dusk_bytes::Error;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut reader = &buf[..];

        let value = u64::from_reader(&mut reader)?;
        let spendable = u64::from_reader(&mut reader)?;

        Ok(Self { value, spendable })
    }

    #[allow(unused_must_use)]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        let mut writer = &mut buf[..];

        writer.write(&self.value.to_bytes());
        writer.write(&self.spendable.to_bytes());

        buf
    }
}
