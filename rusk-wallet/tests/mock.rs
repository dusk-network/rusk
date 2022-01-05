// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Mocks of the traits used in the wallet.

use std::fmt;
use std::sync::Arc;

use dusk_jubjub::BlsScalar;
use dusk_pki::{Ownable, PublicSpendKey, ViewKey};
use dusk_plonk::prelude::{Proof, PublicParameters};
use dusk_poseidon::tree::PoseidonBranch;
use lazy_static::lazy_static;
use phoenix_core::Note;
use rand_core::{CryptoRng, RngCore};
use rusk_abi::RuskModule;
use rusk_vm::{Contract, ContractId, NetworkState};
use rusk_wallet::{NodeClient, Store, UnprovenTransaction, POSEIDON_DEPTH};
use transfer_contract::TransferContract;

const TRANSFER: &[u8] = include_bytes!(
    "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

#[derive(Debug)]
pub struct TestStore {
    seed: [u8; 64],
}

impl TestStore {
    pub fn new<Rng: RngCore + CryptoRng>(rng: &mut Rng) -> Self {
        let mut seed = [0; 64];
        rng.fill_bytes(&mut seed);
        Self { seed }
    }
}

impl Store for TestStore {
    type Error = ();

    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok(self.seed)
    }
}

#[derive(Clone)]
pub struct TestNodeClient {
    network: Arc<NetworkState>,
    transfer: ContractId,
}

impl TestNodeClient {
    pub fn new<Rng: RngCore + CryptoRng>(
        rng: &mut Rng,
        height: u64,
        genesis_psk: &PublicSpendKey,
        initial_balance: u64,
    ) -> Self {
        let mut network = NetworkState::with_block_height(height);

        let rusk_mod = RuskModule::new(&*PP);
        network.register_host_module(rusk_mod);

        let transfer = if initial_balance > 0 {
            let genesis = Note::transparent(rng, genesis_psk, initial_balance);

            TransferContract::try_from(genesis).unwrap()
        } else {
            TransferContract::default()
        };

        let transfer = Contract::new(transfer, TRANSFER.to_vec());
        let transfer = network.deploy(transfer).unwrap();

        let network = Arc::new(network);

        Self { network, transfer }
    }

    fn state(&self) -> TransferContract {
        self.network
            .get_contract_cast_state(&self.transfer)
            .expect("Failed to fetch the state of the contract")
    }

    fn notes(&self, height: u64) -> Vec<Note> {
        self.state()
            .notes_from_height(height)
            .expect("Failed to fetch notes iterator from state")
            .map(|note| *note.expect("Failed to fetch note from canonical"))
            .collect()
    }
}

impl fmt::Debug for TestNodeClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestNodeClient()")
    }
}

impl NodeClient for TestNodeClient {
    type Error = ();

    fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        Ok(self
            .notes(height)
            .iter()
            .filter(|n| vk.owns(n.stealth_address()))
            .copied()
            .collect())
    }

    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        Ok(self.state().notes().inner().root().unwrap_or_default())
    }

    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_DEPTH>, Self::Error> {
        Ok(self
            .state()
            .notes()
            .opening(*note.pos())
            .unwrap_or_else(|_| {
                panic!("Failed to fetch opening of position {:?}", note.pos())
            })
            .unwrap_or_else(|| {
                panic!("Note {:?} not found, opening is undefined!", note.pos())
            }))
    }

    fn request_proof(
        &self,
        _: &UnprovenTransaction,
    ) -> Result<Proof, Self::Error> {
        // FIXME write a mock that actually proves the transaction once the
        //  circuits are fixed by Victor.
        Ok(Proof::default())
    }
}
