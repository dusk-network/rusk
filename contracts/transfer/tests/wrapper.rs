// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::convert::TryFrom;
use transfer_circuits::{ExecuteCircuit, SendToContractTransparentCircuit};
use transfer_contract::{ops, Call, TransferContract};

use canonical::Store;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubScalar;
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::circuit_builder::Circuit;
use dusk_plonk::commitment_scheme::kzg10::srs::PublicParameters;
use dusk_plonk::proof_system::ProverKey;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::{Crossover, Fee, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_vm::{Contract, ContractId, GasMeter, NetworkState, StandardABI};

const TRANSFER_TREE_DEPTH: usize = 17;
const CODE: &'static [u8] = include_bytes!(
    "../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

pub struct TransferWrapper<S: Store> {
    rng: StdRng,
    pp: PublicParameters,
    network: NetworkState<StandardABI<S>, S>,
    contract: ContractId,
    gas: GasMeter,
    genesis_ssk: SecretSpendKey,
}

impl<S: Store> TransferWrapper<S> {
    pub fn new(
        seed: u64,
        block_height: u64,
        initial_balance: u64,
        store: &S,
    ) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let pp = rusk_profile::get_common_reference_string().unwrap();
        let pp = unsafe {
            PublicParameters::from_slice_unchecked(pp.as_slice()).unwrap()
        };

        let mut network =
            NetworkState::<StandardABI<S>, S>::with_block_height(block_height);

        let genesis_ssk = SecretSpendKey::random(&mut rng);
        let genesis_psk = genesis_ssk.public_spend_key();

        let contract = if initial_balance > 0 {
            let genesis =
                Note::transparent(&mut rng, &genesis_psk, initial_balance);

            TransferContract::try_from(genesis).unwrap()
        } else {
            TransferContract::default()
        };
        let contract = Contract::new(contract, CODE.to_vec(), store).unwrap();
        let contract = network.deploy(contract).unwrap();

        let gas = GasMeter::with_limit(1_000);

        Self {
            rng,
            pp,
            network,
            contract,
            gas,
            genesis_ssk,
        }
    }
}

impl<S: Store> TransferWrapper<S> {
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

        note.try_into_fee_crossover(gas_limit, gas_price).unwrap()
    }

    pub fn notes(&mut self, block_height: u64) -> Vec<Note> {
        self.network
            .query::<_, Vec<Note>>(
                self.contract,
                (ops::QR_NOTES_FROM_HEIGHT, block_height),
                &mut self.gas,
            )
            .unwrap()
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

    pub fn balance(&mut self, address: BlsScalar) -> u64 {
        self.network
            .query::<_, u64>(
                self.contract,
                (ops::QR_BALANCE, address),
                &mut self.gas,
            )
            .unwrap()
    }

    pub fn anchor(&mut self) -> BlsScalar {
        self.network
            .query::<_, BlsScalar>(self.contract, ops::QR_ROOT, &mut self.gas)
            .unwrap()
    }

    pub fn opening(&mut self, pos: u64) -> PoseidonBranch<TRANSFER_TREE_DEPTH> {
        self.network
            .query::<_, PoseidonBranch<TRANSFER_TREE_DEPTH>>(
                self.contract,
                (ops::QR_OPENING, pos),
                &mut self.gas,
            )
            .unwrap()
    }

    fn prover_key(rusk_id: &str) -> ProverKey {
        let keys = rusk_profile::keys_for("transker-circuits");
        let (pk, _) = keys.get(rusk_id).unwrap();

        ProverKey::from_bytes(pk.as_slice()).unwrap()
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
    ) -> bool {
        let anchor = self.anchor();

        let mut execute_proof = ExecuteCircuit::default();

        let mut input = 0;
        let nullifiers: Vec<BlsScalar> = inputs
            .iter()
            .zip(inputs_keys.iter())
            .map(|(note, ssk)| {
                let value = note.value(Some(&ssk.view_key())).unwrap();
                input += value;

                let opening = self.opening(note.pos());
                let signature = ExecuteCircuit::sign(&mut self.rng, &ssk, note);
                execute_proof
                    .add_input(&ssk, *note, opening, signature)
                    .unwrap();

                note.gen_nullifier(ssk)
            })
            .collect();

        let output_value = input - gas_limit - value;
        let output = if output_value == 0 {
            vec![]
        } else if output_transparent {
            let note = Note::transparent(&mut self.rng, output, output_value);
            let blinding_factor = note.blinding_factor(None).unwrap();
            execute_proof.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            vec![note]
        } else {
            let blinding_factor = JubJubScalar::random(&mut self.rng);
            let note = Note::obfuscated(
                &mut self.rng,
                output,
                output_value,
                blinding_factor,
            );
            execute_proof.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            vec![note]
        };

        let refund_vk = refund_ssk.view_key();
        let refund_psk = refund_ssk.public_spend_key();
        let (fee, crossover) =
            self.fee_crossover(gas_limit, gas_price, &refund_psk, value);
        execute_proof
            .set_fee_crossover(&fee, &crossover, &refund_vk)
            .unwrap();

        let pk = Self::prover_key(execute_proof.rusk_keys_id());
        // TODO dusk-abi should use the same label
        let spend_proof_execute = execute_proof
            .gen_proof(&self.pp, &pk, b"execute-proof")
            .unwrap();
        let spend_proof_execute = spend_proof_execute.to_bytes().to_vec();

        let signature = SendToContractTransparentCircuit::sign(
            &mut self.rng,
            refund_ssk,
            &fee,
            &crossover,
        );
        let mut stct_proof = SendToContractTransparentCircuit::new(
            &fee, &crossover, &refund_vk, signature,
        )
        .unwrap();
        let pk =
            Self::prover_key(SendToContractTransparentCircuit::rusk_keys_id());
        // TODO dusk-abi should use the same label
        let spend_proof_stct =
            stct_proof.gen_proof(&self.pp, &pk, b"stct-proof").unwrap();
        let spend_proof_stct = spend_proof_stct.to_bytes().to_vec();

        let call = Call::send_to_contract_transparent(
            address,
            value,
            spend_proof_stct,
        )
        .to_execute::<S>(
            self.contract,
            anchor,
            nullifiers,
            fee,
            Some(crossover),
            output,
            spend_proof_execute,
        )
        .unwrap();

        self.network
            .transact::<_, bool>(self.contract, call, &mut self.gas)
            .unwrap()
    }

    pub fn withdraw_from_transparent(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        output: &PublicSpendKey,
        output_transparent: bool,
        gas_limit: u64,
        gas_price: u64,
        address: BlsScalar,
        withdraw_psk: &PublicSpendKey,
        value: u64,
    ) -> bool {
        let anchor = self.anchor();

        let mut execute_proof = ExecuteCircuit::default();

        let mut input = 0;
        let nullifiers: Vec<BlsScalar> = inputs
            .iter()
            .zip(inputs_keys.iter())
            .map(|(note, ssk)| {
                let value = note.value(Some(&ssk.view_key())).unwrap();
                input += value;

                let opening = self.opening(note.pos());
                let signature = ExecuteCircuit::sign(&mut self.rng, &ssk, note);
                execute_proof
                    .add_input(&ssk, *note, opening, signature)
                    .unwrap();

                note.gen_nullifier(ssk)
            })
            .collect();

        let output_value = input - gas_limit;
        let output = if output_value == 0 {
            vec![]
        } else if output_transparent {
            let note = Note::transparent(&mut self.rng, output, output_value);
            let blinding_factor = note.blinding_factor(None).unwrap();
            execute_proof.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            vec![note]
        } else {
            let blinding_factor = JubJubScalar::random(&mut self.rng);
            let note = Note::obfuscated(
                &mut self.rng,
                output,
                output_value,
                blinding_factor,
            );
            execute_proof.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            vec![note]
        };

        let refund_psk =
            SecretSpendKey::random(&mut self.rng).public_spend_key();
        let (fee, _) = self.fee_crossover(gas_limit, gas_price, &refund_psk, 0);
        execute_proof.set_fee(&fee).unwrap();

        let pk = Self::prover_key(execute_proof.rusk_keys_id());
        // TODO dusk-abi should use the same label
        let spend_proof_execute = execute_proof
            .gen_proof(&self.pp, &pk, b"execute-proof")
            .unwrap();
        let spend_proof_execute = spend_proof_execute.to_bytes().to_vec();

        let withdraw = Note::transparent(&mut self.rng, withdraw_psk, value);

        let call = Call::withdraw_from_transparent(address, withdraw)
            .to_execute::<S>(
                self.contract,
                anchor,
                nullifiers,
                fee,
                None,
                output,
                spend_proof_execute,
            )
            .unwrap();

        self.network
            .transact::<_, bool>(self.contract, call, &mut self.gas)
            .unwrap()
    }

    pub fn withdraw_from_transparent_to_contract(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        output: &PublicSpendKey,
        output_transparent: bool,
        gas_limit: u64,
        gas_price: u64,
        from: BlsScalar,
        to: BlsScalar,
        value: u64,
    ) -> bool {
        let anchor = self.anchor();

        let mut execute_proof = ExecuteCircuit::default();

        let mut input = 0;
        let nullifiers: Vec<BlsScalar> = inputs
            .iter()
            .zip(inputs_keys.iter())
            .map(|(note, ssk)| {
                let value = note.value(Some(&ssk.view_key())).unwrap();
                input += value;

                let opening = self.opening(note.pos());
                let signature = ExecuteCircuit::sign(&mut self.rng, &ssk, note);
                execute_proof
                    .add_input(&ssk, *note, opening, signature)
                    .unwrap();

                note.gen_nullifier(ssk)
            })
            .collect();

        let output_value = input - gas_limit;
        let output = if output_value == 0 {
            vec![]
        } else if output_transparent {
            let note = Note::transparent(&mut self.rng, output, output_value);
            let blinding_factor = note.blinding_factor(None).unwrap();
            execute_proof.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            vec![note]
        } else {
            let blinding_factor = JubJubScalar::random(&mut self.rng);
            let note = Note::obfuscated(
                &mut self.rng,
                output,
                output_value,
                blinding_factor,
            );
            execute_proof.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            vec![note]
        };

        let refund_psk =
            SecretSpendKey::random(&mut self.rng).public_spend_key();
        let (fee, _) = self.fee_crossover(gas_limit, gas_price, &refund_psk, 0);
        execute_proof.set_fee(&fee).unwrap();

        let pk = Self::prover_key(execute_proof.rusk_keys_id());
        // TODO dusk-abi should use the same label
        let spend_proof_execute = execute_proof
            .gen_proof(&self.pp, &pk, b"execute-proof")
            .unwrap();
        let spend_proof_execute = spend_proof_execute.to_bytes().to_vec();

        let call = Call::withdraw_from_transparent_to_contract(from, to, value)
            .to_execute::<S>(
                self.contract,
                anchor,
                nullifiers,
                fee,
                None,
                output,
                spend_proof_execute,
            )
            .unwrap();

        self.network
            .transact::<_, bool>(self.contract, call, &mut self.gas)
            .unwrap()
    }
}
