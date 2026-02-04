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

pub use self::execute::feature::Activation as FeatureActivation;
pub use self::execute::{execute, gen_contract_id, Config as ExecutionConfig};
pub use piecrust::{
    CallReceipt, CallTree, CallTreeElem, ContractData, Error, PageOpening,
    Session,
};

/// Contract Metadata
pub struct ContractMetadata {
    /// Contract ID
    pub contract_id: ContractId,
    /// Owner
    pub owner: Vec<u8>,
}

unsafe impl Send for ContractMetadata {}
unsafe impl Sync for ContractMetadata {}

use alloc::vec::Vec;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::path::{Path, PathBuf};
use std::thread;

use dusk_core::abi::{ContractId, Metadata, Query};
use piecrust::{SessionData, VM as PiecrustVM};

use self::host_queries::{
    host_hash, host_keccak256, host_poseidon_hash, host_sha256,
    host_verify_bls, host_verify_bls_multisig, host_verify_groth16_bn254,
    host_verify_kzg_proof, host_verify_plonk, host_verify_schnorr,
    host_secp256k1_recover,
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
pub struct VM {
    inner: PiecrustVM,
    hq_activation: HashMap<String, FeatureActivation>,
}

impl From<PiecrustVM> for VM {
    fn from(piecrust_vm: PiecrustVM) -> Self {
        VM {
            inner: piecrust_vm,
            hq_activation: HashMap::new(),
        }
    }
}

impl Debug for VM {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
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

    /// Sets the activation height for a specific host query.
    ///
    /// This method associates a previously registered host query with a block
    /// height at which it becomes active. Before this activation height,
    /// the host query will be excluded from session execution.
    ///
    /// **Note:** The specified host query must already be registered in the
    /// global host queries registry before calling this method.
    ///
    /// # Arguments
    /// * `host_query` - The name of the host query to activate.
    /// * `activation` - The block height at which the host query becomes
    ///   active.
    ///
    /// # Panics
    /// This method will panic if the provided `host_query` is not already
    /// registered in the global host queries registry.
    ///
    /// # Examples
    /// ```rust
    /// use dusk_vm::VM;
    /// use dusk_vm::FeatureActivation;
    /// use dusk_core::abi::Query;
    ///
    /// let mut vm = VM::ephemeral().unwrap();
    /// vm.with_hq_activation(Query::KECCAK256, FeatureActivation::Height(100));
    /// ```
    pub fn with_hq_activation<S: Into<String>>(
        &mut self,
        host_query: S,
        activation: FeatureActivation,
    ) {
        let host_query = host_query.into();
        if self.inner.host_queries().get(&host_query).is_none() {
            panic!(
                "Host query '{host_query}' must be registered before setting activation"            
            );
        }
        self.hq_activation.insert(host_query, activation);
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
        let mut builder = SessionData::builder()
            .base(base)
            .insert(Metadata::CHAIN_ID, chain_id)?
            .insert(Metadata::BLOCK_HEIGHT, block_height)?;
        // If the block height is greater than 0, exclude host queries
        // that are not yet activated.
        // We don't want to exclude host queries for block height 0 because it's
        // used for query sessions
        if block_height > 0 {
            for (host_query, activation) in &self.hq_activation {
                if !activation.is_active_at(block_height) {
                    builder = builder.exclude_hq(host_query.clone());
                }
            }
        }
        self.inner.session(builder)
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
        self.inner
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
        self.inner.commits()
    }

    /// Deletes a specified commit from the VM.
    ///
    /// # Arguments
    /// * `commit` - The commit to be deleted.
    pub fn delete_commit(&self, root: [u8; 32]) -> Result<(), Error> {
        self.inner.delete_commit(root)
    }

    /// Finalizes a specified commit, applying its state changes permanently.
    ///
    /// # Arguments
    /// * `commit` - The commit to be finalized.
    pub fn finalize_commit(&self, root: [u8; 32]) -> Result<(), Error> {
        self.inner.finalize_commit(root)
    }

    /// Returns the root directory of the VM.
    ///
    /// This is either the directory passed in by using [`Self::new`], or the
    /// temporary directory created using [`Self::ephemeral`].
    pub fn root_dir(&self) -> &Path {
        self.inner.root_dir()
    }

    /// Returns a reference to the synchronization thread.
    pub fn sync_thread(&self) -> &thread::Thread {
        self.inner.sync_thread()
    }

    fn register_host_queries(&mut self) {
        self.inner.register_host_query(Query::HASH, host_hash);
        self.inner
            .register_host_query(Query::POSEIDON_HASH, host_poseidon_hash);
        self.inner
            .register_host_query(Query::VERIFY_PLONK, host_verify_plonk);
        self.inner.register_host_query(
            Query::VERIFY_GROTH16_BN254,
            host_verify_groth16_bn254,
        );
        self.inner
            .register_host_query(Query::VERIFY_SCHNORR, host_verify_schnorr);
        self.inner
            .register_host_query(Query::VERIFY_BLS, host_verify_bls);
        self.inner.register_host_query(
            Query::VERIFY_BLS_MULTISIG,
            host_verify_bls_multisig,
        );
        self.inner
            .register_host_query(Query::KECCAK256, host_keccak256);
        self.inner.register_host_query(Query::SHA256, host_sha256);
        self.inner.register_host_query(
            Query::VERIFY_KZG_PROOF,
            host_verify_kzg_proof,
        );
        self.inner.register_host_query(
            Query::SECP256K1_RECOVER,
            host_secp256k1_recover,
        );
    }

    /// Remove contract
    pub fn remove_3rd_party(
        &self,
        contract_id: ContractId,
    ) -> Result<(), Error> {
        self.inner.remove_module(contract_id)
    }

    /// Recompile contract
    pub fn recompile_3rd_party(
        &self,
        contract_id: ContractId,
    ) -> Result<(), Error> {
        self.inner.recompile_module(contract_id)
    }
}
