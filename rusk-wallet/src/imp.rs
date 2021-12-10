// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Store;

use dusk_bytes::Error as BytesError;
use dusk_pki::SecretSpendKey;
use rand_core::{CryptoRng, Error as RngError, RngCore};

/// The error type returned by this crate.
pub enum Error<S: Store> {
    /// Underlying store error.
    Store(S::Error),
    /// Random number generator error.
    Rng(RngError),
    /// Serialization and deserialization of Dusk types.
    Bytes(BytesError),
}

impl<S: Store> Error<S> {
    /// Returns an error from the underlying store error.
    pub fn from_store_err(se: S::Error) -> Self {
        Self::Store(se)
    }
}

impl<S: Store> From<RngError> for Error<S> {
    fn from(re: RngError) -> Self {
        Self::Rng(re)
    }
}

impl<S: Store> From<BytesError> for Error<S> {
    fn from(be: BytesError) -> Self {
        Self::Bytes(be)
    }
}

/// A wallet implementation.
///
/// This is responsible for holding the keys, and performing operations like
/// creating transactions.
pub struct Wallet<S> {
    store: S,
}

impl<S> Wallet<S> {
    /// Creates a new wallet with the given backing store.
    pub const fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S: Store> Wallet<S> {
    /// Create a secret spend key.
    pub fn create_ssk<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        id: &S::Id,
    ) -> Result<(), Error<S>> {
        let ssk = SecretSpendKey::random(rng);
        self.load_ssk(id, &ssk)
    }

    /// Loads a secret spend key into the wallet.
    pub fn load_ssk(
        &self,
        id: &S::Id,
        ssk: &SecretSpendKey,
    ) -> Result<(), Error<S>> {
        self.store
            .store_key(id, ssk)
            .map_err(Error::from_store_err)?;
        Ok(())
    }

    /// Creates a transfer transaction.
    pub fn create_transfer_tx(&self, id: &S::Id) -> Result<(), Error<S>> {
        todo!()
    }

    /// Creates a stake transaction.
    pub fn create_stake_tx(&self, id: &S::Id) -> Result<(), Error<S>> {
        todo!()
    }

    /// Stops staking for a key.
    pub fn stop_stake(&self) -> Result<(), Error<S>> {
        todo!()
    }

    /// Extends staking for a particular key.
    pub fn extend_stake(&self) -> Result<(), Error<S>> {
        todo!()
    }

    /// Withdraw a key's stake.
    pub fn withdraw_stake(&self) -> Result<(), Error<S>> {
        todo!()
    }

    /// Syncs the wallet with the blocks.
    pub fn sync(&self) -> Result<(), Error<S>> {
        todo!()
    }

    /// Gets the balance of a key.
    pub fn get_balance(&self) -> Result<(), Error<S>> {
        todo!()
    }
}

/// A transaction.
pub struct Transaction {}
