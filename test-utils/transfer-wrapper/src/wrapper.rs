// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use dusk_bls12_381_sign::APK;
use dusk_bytes::Serializable;
use dusk_jubjub::GENERATOR_NUMS_EXTENDED;
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::{Crossover, Fee, Message, Note};
use piecrust::{Error, Session, VM};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::{ModuleError, ModuleId, RawResult};
use stake_contract_types::StakeData;
use transfer_circuits::{
    CircuitInput, DeriveKey, ExecuteCircuit, ExecuteCircuitFourTwo,
    ExecuteCircuitOneTwo, ExecuteCircuitThreeTwo, ExecuteCircuitTwoTwo,
    SendToContractObfuscatedCircuit, SendToContractTransparentCircuit,
    StcoCrossover, StcoMessage,
};
use transfer_contract_types::{
    Stco, Stct, Transaction, TreeLeaf, TRANSFER_TREE_DEPTH,
};

pub struct TransferWrapper<'a> {
    rng: StdRng,
    session: Session<'a>,
    transfer_id: ModuleId,
    stake_id: ModuleId,
    alice: ModuleId,
    bob: ModuleId,
    gas: u64,
    genesis_ssk: SecretSpendKey,
}

#[derive(Default)]
pub struct StakeState<'a> {
    pub stakes: &'a [(APK, StakeData)],
    pub owners: &'a [APK],
    pub allowlist: &'a [APK],
}

impl<'a> TransferWrapper<'a> {
    pub fn new(vm: &'a mut VM, seed: u64, initial_balance: u64) -> Self {
        Self::with_stakes(vm, seed, initial_balance, StakeState::default())
    }

    pub fn with_stakes(
        vm: &'a mut VM,
        seed: u64,
        initial_balance: u64,
        stakes: StakeState,
    ) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session = vm.session();

        let genesis_ssk = SecretSpendKey::random(&mut rng);
        let genesis_psk = genesis_ssk.public_spend_key();

        let transfer_id = rusk_abi::transfer_module();
        let stake_id = rusk_abi::stake_module();

        session
            .deploy_with_id(transfer_id, TRANSFER)
            .expect("Transfer contract should deploy successfully");
        session
            .deploy_with_id(transfer_id, STAKE)
            .expect("Stake contract should deploy successfully");

        // initialize genesis state

        if initial_balance > 0 {
            let genesis_note =
                Note::transparent(&mut rng, &genesis_psk, initial_balance);
            let _: Note = session
                .transact(transfer_id, "push_note", (0, genesis_note))
                .expect("Pushing genesis note should succeed");
        };

        for owner in stakes.owners {
            let stake = StakeData::default();

            let _: Option<StakeData> = session
                .transact(stake_id, "insert_stake", (*owner, stake))
                .expect("Inserting stake should succeed");
            let _: () = session
                .transact(stake_id, "add_owner", *owner)
                .expect("Adding owner to stake contract should succeed");
        }
        for (pk, stake) in stakes.stakes {
            let _: Option<StakeData> = session
                .transact(stake_id, "insert_stake", (*pk, stake.clone()))
                .expect("Inserting stake should succeed");

            if let Some((value, _)) = stake.amount() {
                let _: () = session
                    .transact(
                        transfer_id,
                        "add_module_balance",
                        (stake_id, *value),
                    )
                    .expect("Adding value to module should succeed");
            }
        }
        for allow in stakes.allowlist {
            let _: () = session
                .transact(stake_id, "insert_allowlist", *allow)
                .expect("Inserting to the allowlist should succeed");
        }

        let alice = session
            .deploy(ALICE)
            .expect("Deploying alice should succeed");
        let bob = session.deploy(BOB).expect("Deploying bob should succeed");

        let gas = 1u64;

        Self {
            rng,
            session,
            transfer_id,
            stake_id,
            alice,
            bob,
            gas,
            genesis_ssk,
        }
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    pub fn genesis_identifier(&mut self) -> (SecretSpendKey, Note) {
        let ssk = self.genesis_ssk;
        let vk = ssk.view_key();

        let leaf = self
            .notes_owned_by(0, &vk)
            .first()
            .cloned()
            .expect("Failed to fetch genesis note");

        (ssk, leaf.note)
    }

    pub fn identifier(&mut self) -> (SecretSpendKey, ViewKey, PublicSpendKey) {
        let ssk = SecretSpendKey::random(&mut self.rng);
        let vk = ssk.view_key();
        let psk = ssk.public_spend_key();

        (ssk, vk, psk)
    }

    pub fn transfer(&self) -> ModuleId {
        self.transfer_id
    }

    pub fn stake(&self) -> ModuleId {
        self.stake_id
    }

    pub fn alice(&self) -> ModuleId {
        self.alice
    }

    pub fn bob(&self) -> ModuleId {
        self.bob
    }

    pub fn decrypt_blinder(
        fee: &Fee,
        crossover: &Crossover,
        vk: &ViewKey,
    ) -> JubJubScalar {
        let secret = fee.stealth_address().R() * vk.a();
        let secret = secret.into();

        let data = crossover
            .encrypted_data()
            .decrypt(&secret, crossover.nonce())
            .expect("Failed to decrypt crossover");

        JubJubScalar::from_bytes(&data[1].to_bytes())
            .expect("Failed to decrypt blinder")
    }

    pub fn fee_crossover(
        &mut self,
        gas_limit: u64,
        gas_price: u64,
        refund_psk: &PublicSpendKey,
        value: u64,
    ) -> (Fee, Crossover) {
        let blinder = JubJubScalar::random(&mut self.rng);
        let note = Note::obfuscated(&mut self.rng, refund_psk, value, blinder);

        let (mut fee, crossover) = note.try_into().unwrap();

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        (fee, crossover)
    }

    pub fn fee(
        &mut self,
        gas_limit: u64,
        gas_price: u64,
        refund_psk: &PublicSpendKey,
    ) -> Fee {
        let value = 0;
        let blinding_factor = JubJubScalar::random(&mut self.rng);
        let note =
            Note::obfuscated(&mut self.rng, refund_psk, value, blinding_factor);

        let (mut fee, _) = note.try_into().unwrap();

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        fee
    }

    pub fn generate_note(
        &mut self,
        transparent: bool,
        value: u64,
    ) -> (SecretSpendKey, Note) {
        let (ssk, _, psk) = self.identifier();

        let note = if transparent {
            Note::transparent(&mut self.rng, &psk, value)
        } else {
            let blinder = JubJubScalar::random(&mut self.rng);
            Note::obfuscated(&mut self.rng, &psk, value, blinder)
        };

        (ssk, note)
    }

    fn prover_verifier<C>(circuit_id: &[u8; 32]) -> (Prover<C>, Verifier<C>)
    where
        C: Circuit,
    {
        let keys = rusk_profile::keys_for(circuit_id).unwrap();

        let pk = keys.get_prover().unwrap();
        let vd = keys.get_verifier().unwrap();

        let pk = Prover::try_from_bytes(&pk).unwrap();
        let vd = Verifier::try_from_bytes(&vd).unwrap();

        (pk, vd)
    }

    fn prove_execute(
        &mut self,
        circuit: &ExecuteCircuit,
    ) -> Result<(Proof, Vec<BlsScalar>), PlonkError> {
        match circuit {
            ExecuteCircuit::OneTwo(c) => {
                let (prover, _) = Self::prover_verifier::<ExecuteCircuitOneTwo>(
                    circuit.circuit_id(),
                );
                prover.prove(self.rng(), c)
            }
            ExecuteCircuit::TwoTwo(c) => {
                let (prover, _) = Self::prover_verifier::<ExecuteCircuitTwoTwo>(
                    circuit.circuit_id(),
                );
                prover.prove(self.rng(), c)
            }
            ExecuteCircuit::ThreeTwo(c) => {
                let (prover, _) = Self::prover_verifier::<ExecuteCircuitThreeTwo>(
                    circuit.circuit_id(),
                );
                prover.prove(self.rng(), c)
            }
            ExecuteCircuit::FourTwo(c) => {
                let (prover, _) = Self::prover_verifier::<ExecuteCircuitFourTwo>(
                    circuit.circuit_id(),
                );
                prover.prove(self.rng(), c)
            }
        }
    }

    pub fn notes(&mut self, block_height: u64) -> Vec<TreeLeaf> {
        self.session
            .query(
                self.transfer_id,
                "leaves_in_range",
                (block_height, block_height + 1),
            )
            .expect("Querying existing notes should succeed")
    }

    pub fn notes_owned_by(
        &mut self,
        block_height: u64,
        vk: &ViewKey,
    ) -> Vec<TreeLeaf> {
        self.notes(block_height)
            .iter()
            .filter(|l| vk.owns(l.note.stealth_address()))
            .cloned()
            .collect()
    }

    pub fn anchor(&mut self) -> BlsScalar {
        self.session
            .query(self.transfer_id, "root", ())
            .expect("Querying the anchor should succeed")
    }

    pub fn opening(
        &mut self,
        pos: u64,
    ) -> Option<PoseidonBranch<TRANSFER_TREE_DEPTH>> {
        self.session
            .query(self.transfer_id, "opening", pos)
            .expect("Querying an opening should succeed")
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_execute(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_vk: Option<&ViewKey>,
        output: &PublicSpendKey,
        output_transparent: bool,
        fee: Fee,
        crossover: Option<Crossover>,
        call: Option<(ModuleId, String, Vec<u8>)>,
    ) -> (BlsScalar, Vec<BlsScalar>, Vec<Note>, Proof) {
        self.gas = fee.gas_limit;
        let anchor = self.anchor();

        let (crossover_value, crossover_blinder) = match (refund_vk, crossover)
        {
            (Some(vk), Some(crossover)) => {
                let crossover_note = Note::from((fee, crossover));

                let crossover_value = crossover_note.value(Some(vk)).unwrap();
                let crossover_blinder =
                    crossover_note.blinding_factor(Some(vk)).unwrap();

                (crossover_value, crossover_blinder)
            }

            _ => (0, JubJubScalar::zero()),
        };

        let mut execute_circuit = ExecuteCircuit::new(inputs.len());

        let mut input = 0;

        let nullifiers: Vec<BlsScalar> = inputs
            .iter()
            .zip(inputs_keys.iter())
            .map(|(note, ssk)| {
                let vk = ssk.view_key();

                let value = note.value(Some(&vk)).unwrap();

                input += value;

                note.gen_nullifier(ssk)
            })
            .collect();

        let mut outputs = vec![];
        let output_value =
            input - fee.gas_limit * fee.gas_price - crossover_value;

        if output_value == 0 {
        } else if output_transparent {
            let note = Note::transparent(&mut self.rng, output, output_value);
            let blinding_factor = note.blinding_factor(None).unwrap();

            execute_circuit.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            outputs.push(note);
        } else {
            let blinding_factor = JubJubScalar::random(&mut self.rng);
            let note = Note::obfuscated(
                &mut self.rng,
                output,
                output_value,
                blinding_factor,
            );

            execute_circuit.add_output_with_data(
                note,
                output_value,
                blinding_factor,
            );

            outputs.push(note);
        }

        match crossover {
            Some(crossover) => {
                execute_circuit.set_fee_crossover(
                    &fee,
                    &crossover,
                    crossover_value,
                    crossover_blinder,
                );
            }

            None => {
                execute_circuit.set_fee(&fee);
            }
        }

        let transaction = Transaction {
            anchor,
            nullifiers: nullifiers.clone(),
            outputs,
            fee,
            crossover,
            proof: Proof::default(),
            call,
        };
        let tx_hash = transaction.hash();

        execute_circuit.set_tx_hash(tx_hash);

        inputs
            .iter()
            .zip(inputs_keys.iter())
            .zip(nullifiers.iter())
            .for_each(|((note, ssk), nullifier)| {
                let vk = ssk.view_key();

                let value = note.value(Some(&vk)).unwrap();
                let blinder = note.blinding_factor(Some(&vk)).unwrap();

                let opening = self
                    .opening(*note.pos())
                    .expect("The given input should exist in the tree");

                let sk_r = ssk.sk_r(note.stealth_address());
                let pk_r_p = GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

                let input_signature = ExecuteCircuitOneTwo::input_signature(
                    &mut self.rng,
                    ssk,
                    note,
                    tx_hash,
                );

                let circuit_input = CircuitInput::new(
                    opening,
                    *note,
                    pk_r_p.into(),
                    value,
                    blinder,
                    *nullifier,
                    input_signature,
                );

                execute_circuit.add_input(circuit_input);
            });

        let (proof, _) = self
            .prove_execute(&execute_circuit)
            .expect("Proving the circuit should succeed");

        (anchor, transaction.nullifiers, transaction.outputs, proof)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_vk: &ViewKey,
        output: &PublicSpendKey,
        output_transparent: bool,
        fee: Fee,
        crossover: Option<Crossover>,
        call: Option<(ModuleId, String, Vec<u8>)>,
    ) -> Result<Option<Result<RawResult, ModuleError>>, Error> {
        let (anchor, nullifiers, outputs, spend_proof_execute) = self
            .prepare_execute(
                inputs,
                inputs_keys,
                Some(refund_vk),
                output,
                output_transparent,
                fee,
                crossover,
                call.clone(),
            );

        let transaction = Transaction {
            anchor,
            nullifiers,
            fee,
            crossover,
            outputs,
            proof: spend_proof_execute,
            call,
        };

        Ok(self
            .session
            .transact(self.transfer_id, "execute", transaction)?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_to_contract_transparent(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_ssk: &SecretSpendKey,
        output: &PublicSpendKey,
        output_transparent: bool,
        gas_limit: u64,
        gas_price: u64,
        module: ModuleId,
        value: u64,
    ) -> Result<Option<Result<RawResult, ModuleError>>, Error> {
        let address = rusk_abi::module_to_scalar(&module);
        let refund_vk = refund_ssk.view_key();
        let refund_psk = refund_ssk.public_spend_key();

        let (fee, crossover) =
            self.fee_crossover(gas_limit, gas_price, &refund_psk, value);

        let stct_circuit = {
            let signature = SendToContractTransparentCircuit::sign(
                &mut self.rng,
                refund_ssk,
                &fee,
                &crossover,
                value,
                &address,
            );

            let crossover_note = Note::from((fee, crossover));

            let crossover_value = crossover_note
                .value(Some(&refund_vk))
                .expect("Failed to decrypt value");

            let crossover_blinder = crossover_note
                .blinding_factor(Some(&refund_vk))
                .expect("Failed to decrypt blinder");

            SendToContractTransparentCircuit::new(
                &fee,
                &crossover,
                crossover_value,
                crossover_blinder,
                address,
                signature,
            )
        };

        let (prover, _) =
            Self::prover_verifier::<SendToContractTransparentCircuit>(
                SendToContractTransparentCircuit::circuit_id(),
            );

        let (stct_proof, _) = prover
            .prove(self.rng(), &stct_circuit)
            .expect("Proving STCT should succeed");

        let stct = Stct {
            module,
            value,
            proof: stct_proof,
        };

        let call_bytes = rkyv::to_bytes::<_, 2048>(&stct)
            .expect("serializing Stct should succeed")
            .to_vec();
        let call = (self.transfer_id, String::from("stct"), call_bytes);

        let (anchor, nullifiers, outputs, proof) = self.prepare_execute(
            inputs,
            inputs_keys,
            Some(&refund_vk),
            output,
            output_transparent,
            fee,
            Some(crossover),
            Some(call.clone()),
        );

        let transaction = Transaction {
            anchor,
            nullifiers,
            outputs,
            fee,
            crossover: Some(crossover),
            proof,
            call: Some(call),
        };

        Ok(self
            .session
            .transact(self.transfer_id, "execute", transaction)?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_to_contract_obfuscated(
        &mut self,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_ssk: &SecretSpendKey,
        output: &PublicSpendKey,
        output_transparent: bool,
        gas_limit: u64,
        gas_price: u64,
        module: ModuleId,
        message_psk: &PublicSpendKey,
        value: u64,
    ) -> Result<Option<Result<RawResult, ModuleError>>, Error> {
        let address = rusk_abi::module_to_scalar(&module);
        let refund_vk = refund_ssk.view_key();
        let refund_psk = refund_ssk.public_spend_key();

        let (fee, crossover) =
            self.fee_crossover(gas_limit, gas_price, &refund_psk, value);

        let message_r = JubJubScalar::random(&mut self.rng);
        let message =
            Message::new(&mut self.rng, &message_r, message_psk, value);

        let signature = SendToContractObfuscatedCircuit::sign(
            &mut self.rng,
            refund_ssk,
            &fee,
            &crossover,
            &message,
            &address,
        );

        let stco_message = {
            let message_address = message_psk.gen_stealth_address(&message_r);
            let pk_r = *message_address.pk_r().as_ref();
            let (_, blinder) = message
                .decrypt(&message_r, message_psk)
                .expect("Failed to decrypt message");

            let derive_key = DeriveKey::new(false, message_psk);

            StcoMessage {
                blinder,
                derive_key,
                message,
                pk_r,
                r: message_r,
            }
        };

        let stco_circuit = {
            let crossover_note = Note::from((fee, crossover));

            let crossover_blinder = crossover_note
                .blinding_factor(Some(&refund_vk))
                .expect("Failed to decrypt blinder");

            let stco_crossover =
                StcoCrossover::new(crossover, crossover_blinder);
            SendToContractObfuscatedCircuit::new(
                value,
                stco_message,
                stco_crossover,
                &fee,
                address,
                signature,
            )
        };

        let (prover, _) =
            Self::prover_verifier::<SendToContractObfuscatedCircuit>(
                SendToContractObfuscatedCircuit::circuit_id(),
            );

        let (stco_proof, _) = prover
            .prove(self.rng(), &stco_circuit)
            .expect("Proving STCT should succeed");

        let message_address = message_psk.gen_stealth_address(&message_r);
        let stco = Stco {
            module,
            message,
            message_address,
            proof: stco_proof,
        };

        let call_bytes = rkyv::to_bytes::<_, 2048>(&stco)
            .expect("serializing Stct should succeed")
            .to_vec();
        let call = (self.transfer_id, String::from("stco"), call_bytes);

        let (anchor, nullifiers, outputs, proof) = self.prepare_execute(
            inputs,
            inputs_keys,
            Some(&refund_vk),
            output,
            output_transparent,
            fee,
            Some(crossover),
            Some(call.clone()),
        );

        let transaction = Transaction {
            anchor,
            nullifiers,
            outputs,
            fee,
            crossover: Some(crossover),
            proof,
            call: Some(call),
        };

        Ok(self
            .session
            .transact(self.transfer_id, "execute", transaction)?)
    }
}
