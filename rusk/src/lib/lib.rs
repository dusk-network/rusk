// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod encoding;
pub mod error;
pub mod services;
pub mod state;
pub mod transaction;

use crate::error::Error;
pub use crate::state::RuskState;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;

use microkelvin::{Backend, BackendCtor, DiskBackend, PersistError};

use rusk_abi::{self, RuskModule};
use rusk_vm::{NetworkState, NetworkStateId};

use dusk_plonk::prelude::PublicParameters;

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub static PUB_PARAMS: Lazy<PublicParameters> = Lazy::new(|| unsafe {
    let pp = rusk_profile::get_common_reference_string()
        .expect("Failed to get common reference string");

    PublicParameters::from_slice_unchecked(pp.as_slice())
});

fn disk_backend() -> Result<DiskBackend, PersistError> {
    DiskBackend::new(rusk_profile::get_rusk_state_dir()?)
}

#[derive(Debug, Clone)]
pub struct Rusk {
    pub state_id: Arc<Mutex<NetworkStateId>>,
}

impl Rusk {
    /// Creates a new instance of [`Rusk`]
    pub fn new() -> Result<Rusk> {
        // Register the backend
        Self::with_backend(&BackendCtor::new(disk_backend))
            .or(Err(Error::BackendRegistrationFailed))?;

        let state_id =
            NetworkStateId::read(rusk_profile::get_rusk_state_id_path()?)?;
        let state_id = Arc::new(Mutex::new(state_id));

        Ok(Rusk { state_id })
    }

    /// Creates a new instance of [`Rusk`], deploying a new state based on
    /// the backend given.
    pub fn with_backend<B>(ctor: &BackendCtor<B>) -> Result<Self>
    where
        B: 'static + Backend,
    {
        let state_id = rusk_recovery_tools::state::deploy(ctor)?;
        let state_id = Arc::new(Mutex::new(state_id));

        Ok(Rusk { state_id })
    }

    /// Returns the current state of the network
    pub fn state(&self) -> Result<RuskState> {
        let state_id = *self.state_id.lock();

        let mut network = NetworkState::new()
            .restore(state_id)
            .or(Err(Error::RestoreFailed))?;

        let rusk_mod = RuskModule::new(&PUB_PARAMS);
        network.register_host_module(rusk_mod);

        Ok(RuskState(network))
    }

    /// Persist the current state of the network
    pub fn persist(&self) -> Result<NetworkStateId> {
        let state_id =
            self.persist_with_backend(&BackendCtor::new(disk_backend))?;
        state_id.write(rusk_profile::get_rusk_state_id_path()?)?;
        Ok(state_id)
    }

    /// Persist the current state of the network
    pub fn persist_with_backend<B>(
        &self,
        ctor: &BackendCtor<B>,
    ) -> Result<NetworkStateId>
    where
        B: 'static + Backend,
    {
        let state_id = self.state()?.persist(ctor)?;
        *self.state_id.lock() = state_id;

        Ok(state_id)
    }
}
