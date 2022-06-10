// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
pub use crate::state::RuskState;

pub mod error;
pub mod services;
pub mod state;
pub mod transaction;

use microkelvin::{BackendCtor, DiskBackend, Persistence};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use rusk_abi::{self, RuskModule};
use rusk_vm::{NetworkState, NetworkStateId};

use dusk_plonk::prelude::PublicParameters;

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub static PUB_PARAMS: Lazy<PublicParameters> = Lazy::new(|| unsafe {
    let pp = rusk_profile::get_common_reference_string()
        .expect("Failed to get common reference string");

    PublicParameters::from_slice_unchecked(pp.as_slice())
});

const STREAM_BUF_SIZE: usize = 64;

pub struct RuskBuilder {
    id: Option<NetworkStateId>,
    path: Option<PathBuf>,
    backend: fn() -> BackendCtor<DiskBackend>,
}

impl RuskBuilder {
    pub fn new(backend: fn() -> BackendCtor<DiskBackend>) -> Self {
        Self {
            id: None,
            path: None,
            backend,
        }
    }

    pub fn id(mut self, id: NetworkStateId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn build(self) -> Result<Rusk> {
        let backend = self.backend;

        let network = NetworkState::new();

        let rusk_mod = RuskModule::new(&PUB_PARAMS);
        NetworkState::register_host_module(rusk_mod);

        Persistence::with_backend(&backend(), |_| Ok(()))
            .or(Err(Error::BackendRegistrationFailed))?;

        let path = self.path;
        let id = match (self.id, &path) {
            (Some(id), _) => id,
            (None, Some(path)) => NetworkStateId::read(path)?,
            (None, None) => return Err(Error::BuilderInvalidState),
        };

        let network = network.restore(id).or(Err(Error::RestoreFailed))?;
        let network = Arc::new(Mutex::new(network));

        let rusk = Rusk {
            network,
            backend,
            path,
            stream_buffer_size: STREAM_BUF_SIZE,
        };

        Ok(rusk)
    }

    pub fn path<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.path = Some(path.into());
        self
    }
}

#[derive(Clone)]
pub struct Rusk {
    backend: fn() -> BackendCtor<DiskBackend>,
    network: Arc<Mutex<NetworkState>>,
    path: Option<PathBuf>,
    stream_buffer_size: usize,
}

impl Rusk {
    /// Returns a [`RuskBuilder`]
    pub fn builder(backend: fn() -> BackendCtor<DiskBackend>) -> RuskBuilder {
        RuskBuilder::new(backend)
    }

    /// Returns the current state of the network
    pub fn state(&self, uuid: Uuid) -> Result<RuskState> {
        let network = self.network.clone();

        Ok(RuskState::new(network, uuid))
    }

    /// Persist a state of the network as new state
    pub fn persist(&self, state: &mut RuskState) -> Result<NetworkStateId> {
        let backend = self.backend;
        let network = state.inner();
        let id = network.persist(&backend())?;

        if let Some(path) = &self.path {
            id.write(path)?;
        }

        Ok(id)
    }
}
