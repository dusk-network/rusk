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

pub use dusk_core::transfer::data::gen_contract_id;
pub use piecrust::{
    CallReceipt, CallTree, CallTreeElem, ContractData, Error, PageOpening,
    Session,
};

pub use self::error::ExecutionError;
pub use self::execute::feature::Activation as FeatureActivation;
pub use self::execute::{Config as ExecutionConfig, execute};

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
    hash_host_query, keccak256_host_query, poseidon_hash_host_query,
    secp256k1_recover_host_query, sha256_host_query, verify_bls_host_query,
    verify_bls_multisig_host_query, verify_groth16_bn254_host_query,
    verify_kzg_proof_host_query, verify_plonk_host_query,
    verify_schnorr_host_query,
};

mod error;
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
    /// **Host-query policy:** this does not change the thread-local
    /// [`host_queries::HostQueryPolicy`](crate::host_queries::HostQueryPolicy).
    /// BLS verification inside contract calls therefore defaults to
    /// [`HardFork::PreFork`](crate::host_queries::HardFork::PreFork) until
    /// [`set_host_query_policy`](crate::host_queries::set_host_query_policy)
    /// is called on the same thread. That matches production safety defaults but
    /// means `sign()`-produced signatures need an explicit policy such as
    /// [`HardFork::Aegis`](crate::host_queries::HardFork::Aegis) or
    /// [`HardFork::Boreas`](crate::host_queries::HardFork::Boreas). See
    /// `vm/tests/vm.rs` (`bls_signature`).
    ///
    /// `session` / `genesis_session` `block_height` gates per-query activation
    /// via [`with_hq_activation`](Self::with_hq_activation); it does **not** derive
    /// hardfork policy from height.
    ///
    /// # Returns
    /// A new ephemeral `VM` instance.
    ///
    /// # Errors
    /// If creating a temporary directory fails.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use dusk_vm::{
    ///     host_queries::{
    ///         HardFork, HostQueryPolicy, plonk_version, set_host_query_policy,
    ///     },
    ///     VM,
    /// };
    ///
    /// let vm = VM::ephemeral()?;
    /// let _policy = set_host_query_policy(
    ///     HostQueryPolicy::from_versions(plonk_version(), HardFork::Aegis),
    /// );
    /// let session = vm.genesis_session(0xCA);
    /// # Ok::<(), dusk_vm::Error>(())
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
    ///   created. This height is stored in session metadata and drives
    ///   [`with_hq_activation`](Self::with_hq_activation) exclusions only; it does
    ///   not set [`host_queries::HostQueryPolicy`](crate::host_queries::HostQueryPolicy)
    ///   or BLS hardfork semantics.
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
        self.inner
            .register_host_query(Query::HASH, hash_host_query());
        self.inner.register_host_query(
            Query::POSEIDON_HASH,
            poseidon_hash_host_query(),
        );
        self.inner.register_host_query(
            Query::VERIFY_PLONK,
            verify_plonk_host_query(),
        );
        self.inner.register_host_query(
            Query::VERIFY_GROTH16_BN254,
            verify_groth16_bn254_host_query(),
        );
        self.inner.register_host_query(
            Query::VERIFY_SCHNORR,
            verify_schnorr_host_query(),
        );
        self.inner
            .register_host_query(Query::VERIFY_BLS, verify_bls_host_query());
        self.inner.register_host_query(
            Query::VERIFY_BLS_MULTISIG,
            verify_bls_multisig_host_query(),
        );
        self.inner
            .register_host_query(Query::KECCAK256, keccak256_host_query());
        self.inner
            .register_host_query(Query::SHA256, sha256_host_query());
        self.inner.register_host_query(
            Query::VERIFY_KZG_PROOF,
            verify_kzg_proof_host_query(),
        );
        self.inner.register_host_query(
            Query::SECP256K1_RECOVER,
            secp256k1_recover_host_query(),
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
