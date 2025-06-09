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

pub use self::execute::{execute, gen_contract_id, Config as ExecutionConfig};
pub use piecrust::{
    CallReceipt, CallTree, CallTreeElem, ContractData, Error, PageOpening,
    Session,
};

use alloc::vec::Vec;
use std::fmt::{self, Debug, Formatter};
use std::path::{Path, PathBuf};
use std::thread;

use hex as _;

use dusk_core::abi::{Metadata, Query};
use piecrust::{SessionData, VM as PiecrustVM};

use self::host_queries::{
    host_hash, host_keccak256, host_poseidon_hash, host_verify_bls,
    host_verify_bls_multisig, host_verify_groth16_bn254, host_verify_plonk,
    host_verify_schnorr, host_verify_secp256k1,
};

pub(crate) mod cache;
mod execute;
pub mod host_queries;

/// The Virtual Machine (VM) for executing smart contracts in the Dusk Network.
///
/// The `VM` struct serves as the core for managing the network's state,
/// executing smart contracts, and interfacing with host functions. It supports
/// both persistent and ephemeral sessions for handling transactions, contract
/// queries and contract deployments.
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
    /// Creates a new instance of the virtual machine.
    ///
    /// This method initializes the VM with a given root directory and
    /// registers the necessary host-queries for contract execution.
    ///
    /// # Arguments
    /// * `root_dir` - The path to the root directory for the VM's state
    ///   storage. This directory will be used to save any future session
    ///   commits made by this `VM` instance.
    ///
    /// # Returns
    /// A new `VM` instance.
    ///
    /// # Errors
    /// If the directory contains unparseable or inconsistent data.
    ///
    /// # Examples
    /// ```rust
    /// use dusk_vm::VM;
    ///
    /// let vm = VM::new("/path/to/root_dir");
    /// ```
    pub fn new(
        root_dir: impl AsRef<Path> + Into<PathBuf>,
    ) -> Result<Self, Error> {
        let mut vm: Self = PiecrustVM::new(root_dir)?.into();
        vm.register_host_queries();
        Ok(vm)
    }

    /// Creates an ephemeral VM instance.
    ///
    /// This method initializes a VM that operates in memory without persisting
    /// state. It is useful for testing or temporary computations.
    ///
    /// # Returns
    /// A new ephemeral `VM` instance.
    ///
    /// # Errors
    /// If creating a temporary directory fails.
    ///
    /// # Examples
    /// ```rust
    /// use dusk_vm::VM;
    ///
    /// let vm = VM::ephemeral();
    /// ```
    pub fn ephemeral() -> Result<VM, Error> {
        let mut vm: Self = PiecrustVM::ephemeral()?.into();
        vm.register_host_queries();
        Ok(vm)
    }

    /// Creates a new session for transaction execution.
    ///
    /// This method initializes a session with a specific base state commit,
    /// chain identifier, and block height. Sessions allow for isolated
    /// transaction execution without directly affecting the persistent VM
    /// state until finalized.
    ///
    /// # Arguments
    /// * `base` - A 32-byte array representing the base state from which the
    ///   session begins.
    /// * `chain_id` - The identifier of the network.
    /// * `block_height` - The current block height at which the session is
    ///   created.
    ///
    /// # Returns
    /// A `Result` containing a `Session` instance for executing transactions,
    /// or an error if the session cannot be initialized.
    ///
    /// # Errors
    /// If base commit is provided but does not exist.
    ///
    /// # Examples
    /// ```rust
    /// use dusk_vm::VM;
    ///
    /// const CHAIN_ID: u8 = 42;
    ///
    /// // create a genesis session
    /// let vm = VM::ephemeral().unwrap();
    /// let session = vm.genesis_session(CHAIN_ID);
    ///
    /// // [...] apply changes to the network through the running session
    ///
    /// // commit the changes
    /// let base = session.commit().unwrap();
    ///
    /// // spawn a new session on top of the base-commit
    /// let block_height = 21;
    /// let session = vm.session(base, CHAIN_ID, block_height).unwrap();
    /// ```
    pub fn session(
        &self,
        base: [u8; 32],
        chain_id: u8,
        block_height: u64,
    ) -> Result<Session, Error> {
        self.0.session(
            SessionData::builder()
                .base(base)
                .insert(Metadata::CHAIN_ID, chain_id)?
                .insert(Metadata::BLOCK_HEIGHT, block_height)?,
        )
    }

    /// Initializes a session for setting up the genesis block.
    ///
    /// This method creates a session specifically for defining the genesis
    /// block, which serves as the starting state of the network. The
    /// genesis session uses the specified chain ID.
    ///
    /// # Arguments
    /// * `chain_id` - The identifier of the blockchain chain for which the
    ///   genesis state is initialized.
    ///
    /// # Returns
    /// A `Session` instance for defining the genesis block.
    ///
    /// # Examples
    /// ```rust
    /// use dusk_vm::VM;
    ///
    /// const CHAIN_ID: u8 = 42;
    ///
    /// let vm = VM::ephemeral().unwrap();
    /// let genesis_session = vm.genesis_session(CHAIN_ID);
    /// ```
    pub fn genesis_session(&self, chain_id: u8) -> Session {
        self.0
            .session(
                SessionData::builder()
                    .insert(Metadata::CHAIN_ID, chain_id)
                    .expect("Inserting chain ID in metadata should succeed")
                    .insert(Metadata::BLOCK_HEIGHT, 0)
                    .expect(
                        "Inserting block height in metadata should succeed",
                    ),
            )
            .expect("Creating a genesis session should always succeed")
    }

    /// Retrieves all pending commits in the VM.
    ///
    /// This method fetches unfinalized state changes for inspection or
    /// processing.
    ///
    /// # Returns
    /// A vector of commits.
    pub fn commits(&self) -> Vec<[u8; 32]> {
        self.0.commits()
    }

    /// Deletes a specified commit from the VM.
    ///
    /// # Arguments
    /// * `commit` - The commit to be deleted.
    pub fn delete_commit(&self, root: [u8; 32]) -> Result<(), Error> {
        self.0.delete_commit(root)
    }

    /// Finalizes a specified commit, applying its state changes permanently.
    ///
    /// # Arguments
    /// * `commit` - The commit to be finalized.
    pub fn finalize_commit(&self, root: [u8; 32]) -> Result<(), Error> {
        self.0.finalize_commit(root)
    }

    /// Returns the root directory of the VM.
    ///
    /// This is either the directory passed in by using [`new`], or the
    /// temporary directory created using [`ephemeral`].
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
        self.0.register_host_query(Query::KECCAK256, host_keccak256);
        self.0.register_host_query(
            Query::VERIFY_SECP256K1,
            host_verify_secp256k1,
        );
    }
}
