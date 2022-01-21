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
use crate::state::RuskState;

use microkelvin::{
    Backend, BackendCtor, DiskBackend, PersistError, Persistence,
};

use rusk_abi::{self, RuskModule};
use rusk_vm::{NetworkState, NetworkStateId};

use dusk_plonk::prelude::PublicParameters;

pub type Result<T, E = Error> = core::result::Result<T, E>;

lazy_static::lazy_static! {
    pub(crate) static ref PUB_PARAMS: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string()
            .expect("Failed to get common reference string");

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

fn disk_backend() -> Result<DiskBackend, PersistError> {
    DiskBackend::new(rusk_profile::get_rusk_state_dir()?)
}

#[derive(Debug, Copy, Clone)]
pub struct Rusk {
    state_id: NetworkStateId,
}

impl Rusk {
    /// Creates a new instance of [`Rusk`]
    pub fn new() -> Result<Rusk> {
        // Register the backend
        Persistence::with_backend(&BackendCtor::new(disk_backend), |_| Ok(()))
            .or(Err(Error::BackendRegistrationFailed))?;

        let state_id =
            NetworkStateId::read(rusk_profile::get_rusk_state_id_path()?)?;

        Ok(Rusk { state_id })
    }

    /// Creates a new instance of [`Rusk`], deploying a new state based on
    /// the backend given.
    pub fn with_backend<B>(ctor: &BackendCtor<B>) -> Result<Self>
    where
        B: 'static + Backend,
    {
        let state_id = rusk_recovery_tools::state::deploy(ctor)?;
        Ok(Rusk { state_id })
    }

    /// Returns the current state of the network
    pub fn state(&self) -> Result<RuskState> {
        let mut network = NetworkState::new()
            .restore(self.state_id)
            .or(Err(Error::RestoreFailed))?;

        let rusk_mod = RuskModule::new(&PUB_PARAMS);
        network.register_host_module(rusk_mod);

        Ok(RuskState(network))
    }
}
