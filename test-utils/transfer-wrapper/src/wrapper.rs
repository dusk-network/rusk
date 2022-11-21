// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use alice::Alice;
use bob::Bob;
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_bytes::Serializable;
use dusk_jubjub::GENERATOR_NUMS_EXTENDED;
use dusk_pki::{
    Ownable, PublicKey, PublicSpendKey, SecretSpendKey, ViewKey,
};
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::{Crossover, Fee, Message, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::ModuleId;
// use rusk_vm::{Contract, GasMeter, NetworkState, VMError};
use piecrust::{Session, VM};
use stake_contract::{Stake, StakeContract};
use transfer_circuits::{
    CircuitInput, DeriveKey, ExecuteCircuitOneTwo,
    SendToContractObfuscatedCircuit, SendToContractTransparentCircuit,
    StcoCrossover, StcoMessage,
};
// use transfer_contract::{Call, Error as TransferError, TransferContract};
use transfer_contract::{Error as TransferError, TransferState};
use transfer_contract_types::{Stco2, Stct2, Transaction};

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
    pub stakes: &'a [(BlsPublicKey, Stake)],
    pub owners: &'a [BlsPublicKey],
    pub allowlist: &'a [BlsPublicKey],
}

impl<'a> TransferWrapper<'a> {
    pub fn new(seed: u64, initial_balance: u64) -> Self {
        Self::with_stakes(seed, initial_balance, StakeState::default())
    }

    pub fn with_stakes(
        seed: u64,
        initial_balance: u64,
        stakes: StakeState,
    ) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        // let mut network = NetworkState::new();
        let mut vm = VM::ephemeral().expect("Creating a VM should succeed");
        let mut session = vm.session();

        // let rusk_mod = RuskModule::new(&PP);
        // NetworkState::register_host_module(rusk_mod);

        let genesis_ssk = SecretSpendKey::random(&mut rng);
        let genesis_psk = genesis_ssk.public_spend_key();

        let transfer_id = rusk_abi::transfer_module();
        let stake_id = rusk_abi::stake_module();

        let mut transfer = if initial_balance > 0 {
            let genesis =
                Note::transparent(&mut rng, &genesis_psk, initial_balance);
            TransferState::try_from(genesis).expect(
                "Failed to create a transfer instance from a genesis note",
            )
        } else {
            TransferState::new()
        };

        let stake = {
            let mut contract = StakeContract::default();
            for owner in stakes.owners {
                contract
                    .insert_stake(*owner, stake_contract::Stake::default())
                    .expect("Failed to insert stake");
                contract.add_owner(*owner);
            }
            for (pk, stake) in stakes.stakes {
                contract
                    .insert_stake(*pk, stake.clone())
                    .expect("Failed to insert stake");
                if let Some((value, _)) = stake.amount() {
                    transfer
                        .add_balance(stake_id, *value);
                }
            }
            for allow in stakes.allowlist {
                contract
                    .insert_allowlist(*allow);
            }

            contract
        };

        session
            .deploy_with_id(stake_id, STAKE)
            .expect("Failed to deploy contract");
        session
            .deploy_with_id(transfer_id, TRANSFER)
            .expect("Failed to deploy contract");

        let alice = Self::_deploy(&mut session, ALICE);
        let bob = Self::_deploy(&mut session, BOB);

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

    pub fn deploy<C>(&mut self, bytecode: &[u8]) -> ModuleId {
        Self::_deploy(&mut self.session, bytecode)
    }

    fn _deploy(session: &mut Session, bytecode: &[u8]) -> ModuleId {
        session.deploy(bytecode).expect("Failed to deploy contract")
    }

    // pub fn state<C>(&self, module: &ModuleId) -> C
    // where
    //     C: Deserialize,
    // {
    //     self.session
    //         .get_contract_cast_state(module)
    //         .expect("Failed to fetch the state of the contract")
    // }

    // todo: mock it properly or extend piecrust
    pub fn stake_state(&self) -> StakeContract {
        // self.state(&self.stake_id)
        StakeContract::new()
    }

    // todo: mock it properly or extend piecrust
    pub fn transfer_state(&self) -> TransferState {
        TransferState::new()
    }

    pub fn genesis_identifier(&self) -> (SecretSpendKey, Note) {
        let ssk = self.genesis_ssk;
        let vk = ssk.view_key();

        let note = self
            .notes_owned_by(0, &vk)
            .first()
            .copied()
            .expect("Failed to fetch genesis note");

        (ssk, note)
    }

    pub fn identifier(&mut self) -> (SecretSpendKey, ViewKey, PublicSpendKey) {
        let ssk = SecretSpendKey::random(&mut self.rng);
        let vk = ssk.view_key();
        let psk = ssk.public_spend_key();

        (ssk, vk, psk)
    }

    pub fn alice(&self) -> &ModuleId {
        &self.alice
    }

    pub fn bob(&self) -> &ModuleId {
        &self.bob
    }

    // pub fn tx_ping() -> Transaction {
    //     Transaction::from_canon(&TX_PING)
    // }
    //
    // pub fn tx_withdraw(value: u64, note: Note, proof: Vec<u8>) -> Transaction
    // {     Transaction::from_canon(&(TX_WITHDRAW, value, note, proof))
    // }
    //
    // pub fn tx_withdraw_obfuscated(
    //     message: Message,
    //     message_address: StealthAddress,
    //     change: Message,
    //     change_address: StealthAddress,
    //     note: Note,
    //     proof: Vec<u8>,
    // ) -> Transaction {
    //     Transaction::from_canon(&(
    //         TX_WITHDRAW_OBFUSCATED,
    //         message,
    //         message_address,
    //         change,
    //         change_address,
    //         note,
    //         proof,
    //     ))
    // }
    //
    // pub fn tx_withdraw_to_contract(to: ModuleId, value: u64) -> Transaction {
    //     Transaction::from_canon(&(TX_WITHDRAW_TO_CONTRACT, to, value))
    // }

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

    // pub fn generate_proof<C>(&mut self, mut circuit: C) -> Vec<u8>
    // where
    //     C: Circuit,
    // {
    //     let (pk, _) = Self::circuit_keys(&C::CIRCUIT_ID);
    //
    //     circuit
    //         .prove(&PP, &pk, b"dusk-network")
    //         .expect("Failed to generate proof")
    //         .to_bytes()
    //         .to_vec()
    // }

    pub fn notes(&self, block_height: u64) -> Vec<Note> {
        self.transfer_state()
            .leaves_in_range(block_height..(block_height+1)).iter() // todo: doublecheck if range is correct
            .map(|leaf| leaf.note)
            .collect()
    }

    pub fn notes_owned_by(&self, block_height: u64, vk: &ViewKey) -> Vec<Note> {
        self.notes(block_height)
            .iter()
            .filter(|n| vk.owns(n.stealth_address()))
            .copied()
            .collect()
    }

    pub fn balance(&mut self, address: &ModuleId) -> u64 {
        *self
            .transfer_state()
            .balances()
            .get(address)
            .unwrap()
    }

    pub fn message(
        &self,
        module: &ModuleId,
        pk: &PublicKey,
    ) -> Result<Message, TransferError> {
        self.transfer_state().take_message_from_address_key(module, pk)
    }

    pub fn anchor(&mut self) -> BlsScalar {
        self.transfer_state()
            .root() // todo: make sure this implementation is correct
    }

    pub fn opening(&mut self, pos: u64) -> PoseidonBranch<TRANSFER_TREE_DEPTH> {
        self.transfer_state()
            .opening(pos)
            .unwrap_or_else(|_| {
                panic!("Failed to fetch note of position {:?} for opening", pos)
            })
            .unwrap_or_else(|| {
                panic!("Note {:?} not found, opening is undefined!", pos)
            })
    }

    // todo: almost identical function exists in
    // circuits/transfer/tests/keys/mod.rs remove this duplication
    fn circuit_keys<C>(circuit_id: &[u8; 32]) -> (Prover<C>, Verifier<C>)
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
    ) -> (BlsScalar, Vec<BlsScalar>, Vec<Note>, Vec<u8>) {
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

        let mut execute_proof = ExecuteCircuitOneTwo::default(); // todo: was ExecuteCircuit::default(), is this correct now?
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

            execute_proof
                .add_output_with_data(note, output_value, blinding_factor);

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
                .add_output_with_data(note, output_value, blinding_factor);

            outputs.push(note);
        }

        match crossover {
            Some(crossover) => {
                execute_proof.set_fee_crossover(
                    &fee,
                    &crossover,
                    crossover_value,
                    crossover_blinder,
                );
            }

            None => {
                execute_proof.set_fee(&fee).unwrap();
            }
        }

        let tx_hash = TransferState::tx_hash(
            nullifiers.as_slice(),
            outputs.as_slice(),
            &anchor,
            &fee,
            crossover.as_ref(),
            call.map(|c|c.0),
            call.map(|c|c.2),
        );

        execute_proof.set_tx_hash(tx_hash);

        inputs
            .iter()
            .zip(inputs_keys.iter())
            .zip(nullifiers.iter())
            .for_each(|((note, ssk), nullifier)| {
                let vk = ssk.view_key();

                let value = note.value(Some(&vk)).unwrap();
                let blinder = note.blinding_factor(Some(&vk)).unwrap();

                let opening = self.opening(*note.pos());

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

                execute_proof
                    .add_input(circuit_input);
            });

        let id = ExecuteCircuitOneTwo::circuit_id();
        // let (pk, vd) = Self::circuit_keys(id);
        let (pk, vd) = execute_proof.compile(id, &PP)?;
        let (proof, pi) = execute_proof.prove(, &pk).unwrap();
        // let pi = execute_proof.public_inputs();

        // Sanity check
        // ExecuteCircuitOneTwo::verify(&PP, &vd, &proof,
        // pi.as_slice()).unwrap();
        ExecuteCircuitOneTwo::verify(&vd, &proof, pi.as_slice()).unwrap();

        let proof = proof.to_bytes().to_vec();

        (anchor, nullifiers, outputs, proof)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        block_height: u64,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_vk: &ViewKey,
        output: &PublicSpendKey,
        output_transparent: bool,
        fee: Fee,
        crossover: Option<Crossover>,
        call: Option<(ModuleId, String, Vec<u8>)>,
    ) -> Result<(), Error> {
        let (anchor, nullifiers, outputs, spend_proof_execute) = self
            .prepare_execute(
                inputs,
                inputs_keys,
                Some(refund_vk),
                output,
                output_transparent,
                fee,
                crossover,
                call.as_ref(),
            );

        let transfer_state = TransferState::new();
        let transaction = Transaction {
            anchor,
            nullifiers,
            fee,
            crossover,
            outputs,
            proof: spend_proof_execute,
            call,
        };

        transfer_state.execute(transaction);
        Ok(()) // todo: this impl is temporary and needs to be changed before
               // reviewing

        // self.network.transact::<_, ()>(
        //     self.transfer_id,
        //     block_height,
        //     execute,
        //     &mut self.gas,
        // )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_to_contract_transparent(
        &mut self,
        block_height: u64,
        inputs: &[Note],
        inputs_keys: &[SecretSpendKey],
        refund_ssk: &SecretSpendKey,
        output: &PublicSpendKey,
        output_transparent: bool,
        gas_limit: u64,
        gas_price: u64,
        module: ModuleId,
        value: u64,
    ) -> Result<(), Error> {
        let address = rusk_abi::module_to_scalar(&module);
        let refund_vk = refund_ssk.view_key();
        let refund_psk = refund_ssk.public_spend_key();

        let (fee, crossover) =
            self.fee_crossover(gas_limit, gas_price, &refund_psk, value);

        let mut stct_proof = {
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

        let (pk, _) =
            Self::circuit_keys(&SendToContractTransparentCircuit::CIRCUIT_ID);
        let spend_proof_stct =
            stct_proof.prove(&PP, &pk, b"dusk-network").unwrap();
        let spend_proof_stct = spend_proof_stct.to_bytes().to_vec();

        let call_stct = Stct2 {
            address: module,
            value,
            proof: spend_proof_stct,
        };

        let transaction = call_stct.to_transaction();
        let call = (self.transfer_id, transaction);

        let (anchor, nullifiers, outputs, spend_proof_execute) = self
            .prepare_execute(
                inputs,
                inputs_keys,
                Some(&refund_vk),
                output,
                output_transparent,
                fee,
                Some(crossover),
                Some(&call),
            );

        let call = call_stct
            .to_execute(
                self.transfer_id,
                anchor,
                nullifiers,
                fee,
                Some(crossover),
                outputs,
                spend_proof_execute,
            )
            .unwrap();

        self.network.transact::<_, ()>(
            self.transfer_id,
            block_height,
            call,
            &mut self.gas,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_to_contract_obfuscated(
        &mut self,
        block_height: u64,
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
    ) -> Result<JubJubScalar, Error> {
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

        let mut stco_proof = {
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

        let (pk, _) =
            Self::circuit_keys(&SendToContractObfuscatedCircuit::CIRCUIT_ID);
        let spend_proof_stco =
            stco_proof.prove(&PP, &pk, b"dusk-network").unwrap();
        let spend_proof_stco = spend_proof_stco.to_bytes().to_vec();

        let message_address = message_psk.gen_stealth_address(&message_r);
        let call_stco = Stco2 {
            module,
            message,
            message_address,
            proof: spend_proof_stco,
        };

        let transaction = call_stco.to_transaction();
        let call = (self.transfer_id, transaction);

        let (anchor, nullifiers, outputs, spend_proof_execute) = self
            .prepare_execute(
                inputs,
                inputs_keys,
                Some(&refund_vk),
                output,
                output_transparent,
                fee,
                Some(crossover),
                Some(&call),
            );

        let call = call_stco
            .to_execute(
                self.transfer_id,
                anchor,
                nullifiers,
                fee,
                Some(crossover),
                outputs,
                spend_proof_execute,
            )
            .unwrap();

        self.network.transact::<_, ()>(
            self.transfer_id,
            block_height,
            call,
            &mut self.gas,
        )?;

        Ok(message_r)
    }
}
