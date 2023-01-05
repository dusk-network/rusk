// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The wallet specification.

#![deny(missing_docs)]
#![deny(clippy::all)]
#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(target_family = "wasm")]
mod ffi;

mod imp;
mod tx;

use alloc::vec::Vec;
use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_plonk::proof_system::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Fee, Note};
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};

pub use imp::*;
pub use tx::{Transaction, UnprovenTransaction, UnprovenTransactionInput};

pub use rusk_abi::POSEIDON_TREE_DEPTH;

/// Stores the cryptographic material necessary to derive cryptographic keys.
pub trait Store {
    /// The error type returned from the store.
    type Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error>;

    /// Retrieves a derived secret spend key from the store.
    ///
    /// The provided implementation simply gets the seed and regenerates the key
    /// every time with [`generate_ssk`]. It may be reimplemented to
    /// provide a cache for keys, or implement a different key generation
    /// algorithm.
    fn retrieve_ssk(&self, index: u64) -> Result<SecretSpendKey, Self::Error> {
        let seed = self.get_seed()?;
        Ok(derive_ssk(&seed, index))
    }

    /// Retrieves a derived secret key from the store.
    ///
    /// The provided implementation simply gets the seed and regenerates the key
    /// every time with [`generate_sk`]. It may be reimplemented to
    /// provide a cache for keys, or implement a different key generation
    /// algorithm.
    fn retrieve_sk(&self, index: u64) -> Result<SecretKey, Self::Error> {
        let seed = self.get_seed()?;
        Ok(derive_sk(&seed, index))
    }
}

/// Generates a secret spend key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_ssk(seed: &[u8; 64], index: u64) -> SecretSpendKey {
    let mut hash = Sha256::new();

    hash.update(&seed);
    hash.update(&index.to_le_bytes());
    hash.update(b"SSK");

    let hash = hash.finalize().into();
    let mut rng = ChaCha12Rng::from_seed(hash);

    SecretSpendKey::random(&mut rng)
}

/// Generates a secret key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_sk(seed: &[u8; 64], index: u64) -> SecretKey {
    let mut hash = Sha256::new();

    hash.update(&seed);
    hash.update(&index.to_le_bytes());
    hash.update(b"SK");

    let hash = hash.finalize().into();
    let mut rng = ChaCha12Rng::from_seed(hash);

    SecretKey::random(&mut rng)
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

    /// Requests an STCT proof.
    fn request_stct_proof(
        &self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Result<Proof, Self::Error>;

    /// Request a WFCT proof.
    fn request_wfct_proof(
        &self,
        commitment: JubJubAffine,
        value: u64,
        blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error>;
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

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error>;

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
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Self::Error>;

    /// Queries the node for the stake of a key. If the key has no stake, a
    /// `Default` stake info should be returned.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<StakeInfo, Self::Error>;
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

/// The stake of a particular key.
#[derive(Debug, Default, Hash, Clone, Copy, PartialEq, Eq)]
pub struct StakeInfo {
    /// The value and eligibility of the stake, in that order.
    pub amount: Option<(u64, u64)>,
    /// The reward available for withdrawal.
    pub reward: u64,
    /// Signature counter.
    pub counter: u64,
}

impl Serializable<32> for StakeInfo {
    type Error = dusk_bytes::Error;

    /// Deserializes in the same order as defined in [`to_bytes`]. If the
    /// deserialized value is 0, then `amount` will be `None`. This means that
    /// the eligibility value is left loose, and could be any number when value
    /// is 0.
    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut reader = &buf[..];

        let value = u64::from_reader(&mut reader)?;
        let eligibility = u64::from_reader(&mut reader)?;
        let reward = u64::from_reader(&mut reader)?;
        let counter = u64::from_reader(&mut reader)?;

        let amount = match value > 0 {
            true => Some((value, eligibility)),
            false => None,
        };

        Ok(Self {
            amount,
            reward,
            counter,
        })
    }

    /// Serializes the amount and the eligibility first, and then the reward and
    /// the counter. If `amount` is `None`, and since a stake of no value should
    /// not be possible, the first 16 bytes are filled with zeros.
    #[allow(unused_must_use)]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        let mut writer = &mut buf[..];

        let (value, eligibility) = self.amount.unwrap_or_default();

        writer.write(&value.to_bytes());
        writer.write(&eligibility.to_bytes());
        writer.write(&self.reward.to_bytes());
        writer.write(&self.counter.to_bytes());

        buf
    }
}
