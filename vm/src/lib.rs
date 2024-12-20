// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//![doc = include_str!("../README.md")]

#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

pub use piecrust::{
    CallReceipt, CallTree, CallTreeElem, ContractData, Error, PageOpening,
    Session,
};

use alloc::vec::Vec;
use std::fmt::{self, Debug, Formatter};
use std::path::{Path, PathBuf};
use std::thread;

use dusk_core::abi::{Metadata, Query};
use piecrust::{SessionData, VM as PiecrustVM};

use self::host_queries::{
    host_hash, host_poseidon_hash, host_verify_bls, host_verify_bls_multisig,
    host_verify_groth16_bn254, host_verify_plonk, host_verify_schnorr,
};

pub(crate) mod cache;
pub mod host_queries;

/// Create a new session based on the given `VM`.
pub fn new_session(
    vm: &VM,
    base: [u8; 32],
    chain_id: u8,
    block_height: u64,
) -> Result<Session, Error> {
    vm.session(
        SessionData::builder()
            .base(base)
            .insert(Metadata::CHAIN_ID, chain_id)?
            .insert(Metadata::BLOCK_HEIGHT, block_height)?,
    )
}

/// Create a new genesis session based on the given [`VM`].
pub fn new_genesis_session(vm: &VM, chain_id: u8) -> Session {
    vm.session(
        SessionData::builder()
            .insert(Metadata::CHAIN_ID, chain_id)
            .expect("Inserting chain ID in metadata should succeed")
            .insert(Metadata::BLOCK_HEIGHT, 0)
            .expect("Inserting block height in metadata should succeed"),
    )
    .expect("Creating a genesis session should always succeed")
}

/// Dusk VM is a [`PiecrustVM`] enriched with the host functions specified in
/// Dusk's ABI.
pub struct VM(PiecrustVM);

impl From<PiecrustVM> for VM {
    fn from(piecrust_vm: PiecrustVM) -> Self {
        VM(piecrust_vm)
    }
}

impl Debug for VM {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl VM {
    /// Creates a new `VM`, reading the given directory for existing commits
    /// and bytecode.
    ///
    /// The directory will be used to save any future session commits made by
    /// this `VM` instance.
    ///
    /// # Errors
    /// If the directory contains unparseable or inconsistent data.
    pub fn new(
        root_dir: impl AsRef<Path> + Into<PathBuf>,
    ) -> Result<Self, Error> {
        let mut vm: Self = PiecrustVM::new(root_dir)?.into();
        vm.register_host_queries();
        Ok(vm)
    }

    /// Creates a new `VM` using a new temporary directory.
    ///
    /// Any session commits made by this machine should be considered discarded
    /// once this `VM` instance drops.
    ///
    /// # Errors
    /// If creating a temporary directory fails.
    pub fn ephemeral() -> Result<VM, Error> {
        let mut vm: Self = PiecrustVM::ephemeral()?.into();
        vm.register_host_queries();
        Ok(vm)
    }

    /// Spawn a [`Session`].
    ///
    /// # Errors
    /// If base commit is provided but does not exist.
    ///
    /// [`Session`]: Session
    pub fn session(
        &self,
        data: impl Into<SessionData>,
    ) -> Result<Session, Error> {
        self.0.session(data)
    }

    /// Return all existing commits.
    pub fn commits(&self) -> Vec<[u8; 32]> {
        self.0.commits()
    }

    /// Deletes the given commit from disk.
    pub fn delete_commit(&self, root: [u8; 32]) -> Result<(), Error> {
        self.0.delete_commit(root)
    }

    /// Finalizes the given commit on disk.
    pub fn finalize_commit(&self, root: [u8; 32]) -> Result<(), Error> {
        self.0.finalize_commit(root)
    }

    /// Return the root directory of the virtual machine.
    ///
    /// This is either the directory passed in by using [`new`], or the
    /// temporary directory created using [`ephemeral`].
    ///
    /// [`new`]: VM::new
    /// [`ephemeral`]: VM::ephemeral
    pub fn root_dir(&self) -> &Path {
        self.0.root_dir()
    }

    /// Returns a reference to the synchronization thread.
    pub fn sync_thread(&self) -> &thread::Thread {
        self.0.sync_thread()
    }

    fn register_host_queries(&mut self) {
        self.0.register_host_query(Query::HASH, host_hash);
        self.0
            .register_host_query(Query::POSEIDON_HASH, host_poseidon_hash);
        self.0
            .register_host_query(Query::VERIFY_PLONK, host_verify_plonk);
        self.0.register_host_query(
            Query::VERIFY_GROTH16_BN254,
            host_verify_groth16_bn254,
        );
        self.0
            .register_host_query(Query::VERIFY_SCHNORR, host_verify_schnorr);
        self.0
            .register_host_query(Query::VERIFY_BLS, host_verify_bls);
        self.0.register_host_query(
            Query::VERIFY_BLS_MULTISIG,
            host_verify_bls_multisig,
        );
    }
}

#[cfg(test)]
mod tests {
    // the `unused_crate_dependencies` lint complains for dev-dependencies that
    // are only used in integration tests, so adding this work-around here
    use ff as _;
    use once_cell as _;
    use rand as _;
}
