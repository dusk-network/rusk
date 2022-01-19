// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use alloc::vec::Vec;
use canonical::{Canon, Sink, Source};
use dusk_abi::{ContractId, ContractState, ReturnValue, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::PublicKey;
use dusk_schnorr::Signature;
use phoenix_core::Note;
use rusk_abi::PaymentInfo;
use transfer_contract::Call;

pub const QR_GET_STAKE: u8 = 0x00;

pub const TX_STAKE: u8 = 0x00;
pub const TX_EXTEND: u8 = 0x01;
pub const TX_WITHDRAW: u8 = 0x02;
pub const TX_SLASH: u8 = 0x03;

const PAGE_SIZE: usize = 1024 * 32;

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    let mut source = Source::new(bytes);

    let contract =
        StakeContract::decode(&mut source).expect("Failed to read state");
    let qid = u8::decode(&mut source).expect("Failed to read query ID");

    let ret = match qid {
        rusk_abi::PAYMENT_INFO => {
            ReturnValue::from_canon(&PaymentInfo::Transparent(None))
        }

        QR_GET_STAKE => {
            let pk: PublicKey =
                Canon::decode(&mut source).expect("Failed to parse PK");

            let stake = contract.get_stake(&pk);
            ReturnValue::from_canon(&stake)
        }

        _ => panic!("Undefined query ID"),
    };

    let mut sink = Sink::new(&mut bytes[..]);

    ret.encode(&mut sink);
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    let mut source = Source::new(bytes);

    let mut contract =
        StakeContract::decode(&mut source).expect("Failed to read state");
    let tid = u8::decode(&mut source).expect("Failed to read tx ID");

    match tid {
        TX_STAKE => {
            let (pk, value, spend_proof): (PublicKey, u64, Vec<u8>) =
                Canon::decode(&mut source).expect("Failed to parse arguments");

            contract.stake(pk, value, spend_proof);
        }

        TX_EXTEND => {
            let (pk, signature): (PublicKey, Signature) =
                Canon::decode(&mut source).expect("Failed to parse arguments");

            contract.extend(pk, signature);
        }

        TX_WITHDRAW => {
            let (pk, signature, note, withdraw_proof): (
                PublicKey,
                Signature,
                Note,
                Vec<u8>,
            ) = Canon::decode(&mut source).expect("Failed to parse arguments");

            contract.withdraw(pk, signature, note, withdraw_proof);
        }

        _ => panic!("Tx id not implemented"),
    }

    let mut sink = Sink::new(&mut bytes[..]);

    ContractState::from_canon(&contract).encode(&mut sink);
    ReturnValue::from_canon(&true).encode(&mut sink);
}

impl StakeContract {
    fn push_stake(&mut self, pk: PublicKey, stake: Stake) -> bool {
        let pk = pk.to_bytes();

        self.staked
            .insert(pk, stake)
            .expect("Failed to access staked map")
            .is_some()
    }

    pub fn get_stake(&self, pk: &PublicKey) -> Stake {
        *self
            .staked
            .get(&pk.to_bytes())
            .expect("Failed to access the staked map")
            .expect("The provided key is not staked")
    }

    pub fn stake(&mut self, pk: PublicKey, value: u64, spend_proof: Vec<u8>) {
        if value < MINIMUM_STAKE {
            panic!("The staked value is not within range!");
        }

        let block_height = dusk_abi::block_height() as u32;
        let epoch = EPOCH - block_height % EPOCH;
        let eligibility = block_height + MATURITY + epoch;
        let expiration = block_height + MATURITY + VALIDITY + epoch;

        let address = dusk_abi::callee();
        if address != rusk_abi::transfer_contract() {
            panic!("Can only be called from the transfer contract");
        }

        let stake = Stake::new(value, eligibility, expiration);

        if self.push_stake(pk, stake) {
            panic!(
                "The provided key is already staked! It can only be extended"
            );
        }

        let call =
            Call::send_to_contract_transparent(address, value, spend_proof);
        let call = Transaction::from_canon(&call);
        let transfer = rusk_abi::transfer_contract();

        dusk_abi::transact_raw(self, &transfer, &call, 0)
            .expect("Failed to send note to contract");
    }

    pub fn extend(&mut self, pk: PublicKey, signature: Signature) {
        let mut stake = self.get_stake(&pk);

        let block_height = dusk_abi::block_height() as u32;
        let expiration = stake.expiration();

        if block_height + MATURITY < expiration {
            panic!("The provided stake is not matured yet");
        }

        if block_height >= expiration {
            panic!("The provided stake is expired!");
        }

        let message = BlsScalar::from(expiration as u64);
        rusk_abi::verify_schnorr_sign(signature, pk, message);

        stake.extend();
        self.push_stake(pk, stake);
    }

    pub fn withdraw(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        note: Note,
        withdraw_proof: Vec<u8>,
    ) {
        let stake = self.get_stake(&pk);
        let block_height = dusk_abi::block_height() as u32;

        if block_height < stake.expiration() + EPOCH {
            panic!("The provided stake is expired");
        }

        let message = BlsScalar::from(stake.expiration() as u64);
        rusk_abi::verify_schnorr_sign(signature, pk, message);

        self.staked
            .remove(&pk.to_bytes())
            .expect("Failed to remove stake");

        let value = stake.value();
        let call = Call::withdraw_from_transparent(value, note, withdraw_proof);
        let call = Transaction::from_canon(&call);
        let transfer = rusk_abi::transfer_contract();

        dusk_abi::transact_raw(self, &transfer, &call, 0)
            .expect("Failed to withdraw note from contract");
    }
}
