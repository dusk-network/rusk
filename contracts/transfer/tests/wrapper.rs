// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::convert::{TryFrom, TryInto};
use transfer_circuits::{ExecuteCircuit, SendToContractTransparentCircuit};
use transfer_contract::{Call, TransferContract};

use dusk_bytes::Serializable;
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_poseidon::tree::PoseidonBranch;
use lazy_static::lazy_static;
use phoenix_core::{Crossover, Fee, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk::vm::{Contract, ContractId, GasMeter, NetworkState, VMError};
use rusk_abi::RuskModule;

use dusk_plonk::prelude::*;

const TRANSFER_TREE_DEPTH: usize = 17;
const CODE: &'static [u8] = include_bytes!(
    "../../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

pub struct TransferWrapper {
    rng: StdRng,
    network: NetworkState,
    contract: ContractId,
    gas: GasMeter,
    genesis_ssk: SecretSpendKey,
}

impl TransferWrapper {
    pub fn new(seed: u64, block_height: u64, initial_balance: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut network = NetworkState::with_block_height(block_height);

        let rusk_mod = RuskModule::new(&*PP);
        network.register_host_module(rusk_mod);

        let genesis_ssk = SecretSpendKey::random(&mut rng);
        let genesis_psk = genesis_ssk.public_spend_key();

        let contract = if initial_balance > 0 {
            let genesis =
                Note::transparent(&mut rng, &genesis_psk, initial_balance);

            TransferContract::try_from(genesis).unwrap()
        } else {
            TransferContract::default()
        };
        let contract = Contract::new(contract, CODE.to_vec());
        let contract = network.deploy(contract).unwrap();

        let gas = GasMeter::with_limit(1_000);

        Self {
            rng,
            network,
            contract,
            gas,
            genesis_ssk,
        }
    }

    pub fn state(&self) -> TransferContract {
        self.network
            .get_contract_cast_state(&self.contract)
            .expect("Failed to fetch the state of the contract")
    }

    pub fn genesis_identifier(
        &self,
    ) -> (SecretSpendKey, ViewKey, PublicSpendKey) {
        let ssk = self.genesis_ssk;
        let vk = ssk.view_key();
        let psk = ssk.public_spend_key();

        (ssk, vk, psk)
    }

    pub fn identifier(&mut self) -> (SecretSpendKey, ViewKey, PublicSpendKey) {
        let ssk = SecretSpendKey::random(&mut self.rng);
        let vk = ssk.view_key();
        let psk = ssk.public_spend_key();

        (ssk, vk, psk)
    }

    pub fn address(&mut self) -> BlsScalar {
        BlsScalar::random(&mut self.rng)
    }

    pub fn fee_crossover(
        &mut self,
        gas_limit: u64,
        gas_price: u64,
        refund_psk: &PublicSpendKey,
        value: u64,
    ) -> (Fee, Crossover) {
        let blinding_factor = JubJubScalar::random(&mut self.rng);
        let note =
            Note::obfuscated(&mut self.rng, refund_psk, value, blinding_factor);

        let (mut fee, crossover) = note.try_into().unwrap();
        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        (fee, crossover)
    }

    pub fn notes(&mut self, block_height: u64) -> Vec<Note> {
        self.state()
            .notes_from_height(block_height)
            .expect("Failed to fetch notes iterator from state")
            .map(|note| {
                note.expect("Failed to fetch note from canonical").clone()
            })
            .collect()
    }

    pub fn notes_owned_by(
        &mut self,
        block_height: u64,
        vk: &ViewKey,
    ) -> Vec<Note> {
        self.notes(block_height)
            .iter()
            .filter(|n| vk.owns(n.stealth_address()))
            .map(|n| n.clone())
            .collect()
    }

    pub fn balance(&mut self, address: &BlsScalar) -> u64 {
        *self
            .state()
            .balances()
            .get(address)
            .unwrap()
            .as_deref()
            .unwrap_or(&0)
    }

    pub fn anchor(&mut self) -> BlsScalar {
        self.state().notes().inner().root().unwrap_or_default()
    }

    pub fn opening(&mut self, pos: u64) -> PoseidonBranch<TRANSFER_TREE_DEPTH> {
        self.state()
            .notes()
            .opening(pos)
            .expect(
                format!(
                    "Failed to fetch note of position {:?} for opening",
                    pos
                )
                .as_str(),
            )
            .expect(
                format!("Note {:?} not found, opening is undefined!", pos)
                    .as_str(),
            )
    }

    fn circuit_keys(circuit_id: &[u8; 32]) -> (ProverKey, VerifierData) {
        let keys = rusk_profile::keys_for(circuit_id).unwrap();

        let pk = keys.get_prover().unwrap();
        let vd = keys.get_verifier().unwrap();

        let pk = ProverKey::from_slice(pk.as_slice()).unwrap();
        let vd = VerifierData::from_slice(vd.as_slice()).unwrap();

        (pk, vd)
    }

    fn prepare_execute(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_vk: Option<&ViewKey>,
        output: &PublicSpendKey,
        output_transparent: bool,
        gas_limit: u64,
        gas_price: u64,
        crossover_value: u64,
    ) -> (
        BlsScalar,
        Vec<BlsScalar>,
        Fee,
        Option<Crossover>,
        Vec<Note>,
        Vec<u8>,
    ) {
        let anchor = self.anchor();

        let mut execute_proof = ExecuteCircuit::default();
        let mut input = 0;

        let nullifiers: Vec<BlsScalar> = inputs
            .iter()
            .zip(inputs_keys.iter())
            .map(|(note, ssk)| {
                let value = note.value(Some(&ssk.view_key())).unwrap();
                input += value;

                let opening = self.opening(*note.pos());
                let signature = ExecuteCircuit::sign(&mut self.rng, &ssk, note);
                execute_proof
                    .add_input(&ssk, *note, opening, signature)
                    .unwrap();

                note.gen_nullifier(ssk)
            })
            .collect();

        let mut outputs = vec![];
        let output_value = input - gas_limit - crossover_value;

        if output_value == 0 {
        } else if output_transparent {
            let note = Note::transparent(&mut self.rng, output, output_value);
            let blinding_factor = note.blinding_factor(None).unwrap();

            execute_proof
                .add_output_with_data(note, output_value, blinding_factor)
                .unwrap();

            outputs.push(note);
        } else {
            let blinding_factor = JubJubScalar::random(&mut self.rng);
            let note = Note::obfuscated(
                &mut self.rng,
                output,
                output_value,
                blinding_factor,
            );

            execute_proof
                .add_output_with_data(note, output_value, blinding_factor)
                .unwrap();

            outputs.push(note);
        }

        let (fee, crossover) = match refund_vk {
                Some(vk) => {
                    let psk = vk.public_spend_key();
                    let (fee, crossover) = self.fee_crossover(
                        gas_limit,
                        gas_price,
                        &psk,
                        crossover_value,
                    );

                    execute_proof
                        .set_fee_crossover(&fee, &crossover, vk)
                        .unwrap();

                    (fee, Some(crossover))
                }

                None if crossover_value > 0 => panic!("The refund SSK is mandatory for transactions with a crossover value!"),

                None => {
                    let psk =
                        SecretSpendKey::random(&mut self.rng).public_spend_key();
                    let (fee, _) =
                        self.fee_crossover(gas_limit, gas_price, &psk, 0);
                    execute_proof.set_fee(&fee).unwrap();

                    (fee, None)
                }
            };

        let id = execute_proof.circuit_id();
        let (pk, vd) = Self::circuit_keys(id);

        let proof =
            execute_proof.gen_proof(&*PP, &pk, b"dusk-network").unwrap();
        let pi = execute_proof.public_inputs();

        // Sanity check
        circuit::verify_proof(
            &*PP,
            vd.key(),
            &proof,
            pi.as_slice(),
            vd.pi_pos(),
            b"dusk-network",
        )
        .unwrap();

        let proof = proof.to_bytes().to_vec();

        (anchor, nullifiers, fee, crossover, outputs, proof)
    }

    pub fn send_to_contract_transparent(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_ssk: &SecretSpendKey,
        output: &PublicSpendKey,
        output_transparent: bool,
        gas_limit: u64,
        gas_price: u64,
        address: BlsScalar,
        value: u64,
    ) -> Result<(), VMError> {
        let refund_vk = refund_ssk.view_key();
        let (anchor, nullifiers, fee, crossover, outputs, spend_proof_execute) =
            self.prepare_execute(
                inputs,
                inputs_keys,
                Some(&refund_vk),
                output,
                output_transparent,
                gas_limit,
                gas_price,
                value,
            );

        let crossover = crossover.unwrap();
        let signature = SendToContractTransparentCircuit::sign(
            &mut self.rng,
            refund_ssk,
            &fee,
            &crossover,
            value,
            &address,
        );
        let mut stct_proof = SendToContractTransparentCircuit::new(
            fee, crossover, &refund_vk, address, signature,
        )
        .unwrap();
        let (pk, _) =
            Self::circuit_keys(&SendToContractTransparentCircuit::CIRCUIT_ID);
        let spend_proof_stct =
            stct_proof.gen_proof(&*PP, &pk, b"dusk-network").unwrap();
        let spend_proof_stct = spend_proof_stct.to_bytes().to_vec();

        let call = Call::send_to_contract_transparent(
            address,
            value,
            spend_proof_stct,
        )
        .to_execute(
            self.contract,
            anchor,
            nullifiers,
            fee,
            Some(crossover),
            outputs,
            spend_proof_execute,
        )
        .unwrap();

        self.network
            .transact::<_, ()>(self.contract, call, &mut self.gas)
    }
}
