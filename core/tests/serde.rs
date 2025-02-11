// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(feature = "serde")]

use bls12_381_bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::signatures::schnorr::PublicKey as NotePublicKey;
use dusk_core::signatures::schnorr::SecretKey as NoteSecretKey;
use dusk_core::stake::{
    Reward, RewardReason, SlashEvent, StakeEvent, StakeFundOwner, StakeKeys,
};
use dusk_core::transfer::moonlight::Fee as MoonlightFee;
use dusk_core::transfer::phoenix::Fee as PhoenixFee;
use dusk_core::transfer::withdraw::WithdrawReceiver;
use dusk_core::transfer::WithdrawEvent;
use dusk_core::transfer::{
    ContractToAccountEvent, ContractToContractEvent, ConvertEvent,
    DepositEvent, MoonlightTransactionEvent, PhoenixTransactionEvent,
};
use dusk_core::{BlsScalar, JubJubScalar};
use ff::Field;
use phoenix_core::{
    Note, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey, Sender,
};
use piecrust_uplink::{ContractId, CONTRACT_ID_BYTES};
use rand::rngs::StdRng;
use rand::Rng;
use rand::{RngCore, SeedableRng};

#[test]
fn serde_stake_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut contract_id_bytes = [0; CONTRACT_ID_BYTES];
    rng.fill_bytes(&mut contract_id_bytes);
    let pk = AccountPublicKey::from(&AccountSecretKey::random(&mut rng));
    let owner1 = StakeFundOwner::Account(pk);
    let owner2 =
        StakeFundOwner::Contract(ContractId::from_bytes(contract_id_bytes));
    let event1 = StakeEvent {
        keys: StakeKeys::new(pk, owner1),
        value: rng.next_u64(),
        locked: rng.next_u64(),
    };
    let event2 = StakeEvent {
        keys: StakeKeys::new(pk, owner2),
        value: rng.next_u64(),
        locked: rng.next_u64(),
    };

    let ser1 = serde_json::to_string(&event1).unwrap();
    let ser2 = serde_json::to_string(&event2).unwrap();
    let deser1 = serde_json::from_str(&ser1).unwrap();
    let deser2 = serde_json::from_str(&ser2).unwrap();

    assert_eq!(event1, deser1);
    assert_eq!(event2, deser2);
    assert_ne!(deser1, deser2);
}

#[test]
fn serde_slash_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let event = SlashEvent {
        account: AccountPublicKey::from(&AccountSecretKey::random(&mut rng)),
        value: rng.next_u64(),
        next_eligibility: rng.next_u64(),
    };
    let ser = serde_json::to_string(&event).unwrap();
    let deser = serde_json::from_str(&ser).unwrap();
    assert_eq!(event, deser);
}

#[test]
fn serde_reward() {
    use RewardReason::*;
    let mut rng = StdRng::seed_from_u64(42);
    let account = AccountPublicKey::from(&AccountSecretKey::random(&mut rng));
    let mut events = vec![];
    for reason in [GeneratorExtra, GeneratorFixed, Voter, Other] {
        events.push(Reward {
            account,
            value: rng.next_u64(),
            reason,
        });
    }
    let mut desers = vec![];
    for event in &events {
        let ser = serde_json::to_string(event).unwrap();
        let deser = serde_json::from_str(&ser).unwrap();
        desers.push(deser);
    }
    assert_eq!(events, desers);
}

#[test]
fn serde_withdraw_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut contract_id_bytes = [0; CONTRACT_ID_BYTES];
    rng.fill_bytes(&mut contract_id_bytes);
    let sender = ContractId::from_bytes(contract_id_bytes);
    let scalar = JubJubScalar::random(&mut rng);
    let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
    let stealth_addr = pk.gen_stealth_address(&scalar);

    let event1 = WithdrawEvent {
        sender,
        receiver: WithdrawReceiver::Moonlight(AccountPublicKey::from(
            &AccountSecretKey::random(&mut rng),
        )),
        value: rng.next_u64(),
    };
    let event2 = WithdrawEvent {
        sender,
        receiver: WithdrawReceiver::Phoenix(stealth_addr),
        value: rng.next_u64(),
    };

    let ser1 = serde_json::to_string(&event1).unwrap();
    let ser2 = serde_json::to_string(&event2).unwrap();
    let deser1 = serde_json::from_str(&ser1).unwrap();
    let deser2 = serde_json::from_str(&ser2).unwrap();

    assert_eq!(event1, deser1);
    assert_eq!(event2, deser2);
    assert_ne!(deser1, deser2);
}

#[test]
fn serde_convert_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let scalar = JubJubScalar::random(&mut rng);
    let account_pk =
        AccountPublicKey::from(&AccountSecretKey::random(&mut rng));
    let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
    let stealth_addr = pk.gen_stealth_address(&scalar);

    let event1 = ConvertEvent {
        sender: None,
        receiver: WithdrawReceiver::Moonlight(account_pk.clone()),
        value: rng.next_u64(),
    };
    let event2 = ConvertEvent {
        sender: Some(account_pk),
        receiver: WithdrawReceiver::Phoenix(stealth_addr),
        value: rng.next_u64(),
    };

    let ser1 = serde_json::to_string(&event1).unwrap();
    let ser2 = serde_json::to_string(&event2).unwrap();
    let deser1 = serde_json::from_str(&ser1).unwrap();
    let deser2 = serde_json::from_str(&ser2).unwrap();

    assert_eq!(event1, deser1);
    assert_eq!(event2, deser2);
    assert_ne!(deser1, deser2);
}

#[test]
fn serde_deposit_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut contract_id_bytes = [0; CONTRACT_ID_BYTES];
    rng.fill_bytes(&mut contract_id_bytes);
    let pk = AccountPublicKey::from(&AccountSecretKey::random(&mut rng));
    let contract_id = ContractId::from_bytes(contract_id_bytes);

    let event1 = DepositEvent {
        sender: None,
        receiver: contract_id,
        value: rng.next_u64(),
    };
    let event2 = DepositEvent {
        sender: Some(pk),
        receiver: contract_id,
        value: rng.next_u64(),
    };

    let ser1 = serde_json::to_string(&event1).unwrap();
    let ser2 = serde_json::to_string(&event2).unwrap();
    let deser1 = serde_json::from_str(&ser1).unwrap();
    let deser2 = serde_json::from_str(&ser2).unwrap();

    assert_eq!(event1, deser1);
    assert_eq!(event2, deser2);
    assert_ne!(deser1, deser2);
}

#[test]
fn serde_contract_to_contract_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut contract_id_bytes1 = [0; CONTRACT_ID_BYTES];
    let mut contract_id_bytes2 = [0; CONTRACT_ID_BYTES];
    rng.fill_bytes(&mut contract_id_bytes1);
    rng.fill_bytes(&mut contract_id_bytes2);
    let sender = ContractId::from_bytes(contract_id_bytes1);
    let receiver = ContractId::from_bytes(contract_id_bytes2);

    let event = ContractToContractEvent {
        sender,
        receiver,
        value: rng.next_u64(),
    };

    let ser = serde_json::to_string(&event).unwrap();
    let deser = serde_json::from_str(&ser).unwrap();
    assert_eq!(event, deser);
}

#[test]
fn serde_contract_to_account_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut contract_id_bytes = [0; CONTRACT_ID_BYTES];
    rng.fill_bytes(&mut contract_id_bytes);
    let sender = ContractId::from_bytes(contract_id_bytes);
    let receiver = AccountPublicKey::from(&AccountSecretKey::random(&mut rng));

    let event = ContractToAccountEvent {
        sender,
        receiver,
        value: rng.next_u64(),
    };

    let ser = serde_json::to_string(&event).unwrap();
    let deser = serde_json::from_str(&ser).unwrap();
    assert_eq!(event, deser);
}

fn rand_note() -> Note {
    let mut rng = StdRng::seed_from_u64(42);
    let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
    let blinder = JubJubScalar::random(&mut rng);
    let sender_blinder = [
        JubJubScalar::random(&mut rng),
        JubJubScalar::random(&mut rng),
    ];
    Note::obfuscated(&mut rng, &pk, &pk, 42, blinder, sender_blinder)
}

#[test]
fn serde_phoenix_transaction_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut nullifiers = vec![];
    for _ in 0..rng.gen_range(0..10) {
        nullifiers.push(BlsScalar::random(&mut rng));
    }
    let mut notes = vec![];
    for _ in 0..rng.gen_range(0..10) {
        notes.push(rand_note());
    }
    let mut memo = vec![0; 50];
    rng.fill_bytes(&mut memo);

    let event1 = PhoenixTransactionEvent {
        nullifiers: nullifiers.clone(),
        notes: notes.clone(),
        memo: memo.clone(),
        gas_spent: rng.next_u64(),
        refund_note: None,
    };
    let event2 = PhoenixTransactionEvent {
        nullifiers: nullifiers.clone(),
        notes: notes.clone(),
        memo: memo.clone(),
        gas_spent: rng.next_u64(),
        refund_note: Some(rand_note()),
    };

    let ser1 = serde_json::to_string(&event1).unwrap();
    let ser2 = serde_json::to_string(&event2).unwrap();
    let deser1 = serde_json::from_str(&ser1).unwrap();
    let deser2 = serde_json::from_str(&ser2).unwrap();

    assert_eq!(event1, deser1);
    assert_eq!(event2, deser2);
    assert_ne!(deser1, deser2);
}

#[test]
fn serde_moonlight_transaction_event() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut memo = vec![0; 50];
    rng.fill_bytes(&mut memo);
    let pk = AccountPublicKey::from(&AccountSecretKey::random(&mut rng));

    let event1 = MoonlightTransactionEvent {
        sender: pk.clone(),
        receiver: Some(pk.clone()),
        value: rng.next_u64(),
        memo: memo.clone(),
        gas_spent: rng.next_u64(),
        refund_info: Some((pk, rng.next_u64())),
    };
    let event2 = MoonlightTransactionEvent {
        sender: pk,
        receiver: None,
        value: rng.next_u64(),
        memo,
        gas_spent: rng.next_u64(),
        refund_info: None,
    };

    let ser1 = serde_json::to_string(&event1).unwrap();
    let ser2 = serde_json::to_string(&event2).unwrap();
    let deser1 = serde_json::from_str(&ser1).unwrap();
    let deser2 = serde_json::from_str(&ser2).unwrap();
    assert_eq!(event1, deser1);
    assert_eq!(event2, deser2);
    assert_ne!(deser1, deser2);
}

#[test]
fn serde_moonlight_fee() {
    let mut rng = StdRng::seed_from_u64(42);
    let fee = MoonlightFee {
        gas_limit: rng.next_u64(),
        gas_price: rng.next_u64(),
        refund_address: AccountPublicKey::from(&AccountSecretKey::random(
            &mut rng,
        )),
    };

    let ser = serde_json::to_string(&fee).unwrap();
    let deser = serde_json::from_str(&ser).unwrap();
    assert_eq!(fee, deser);
}

#[test]
fn serde_phoenix_fee() {
    let mut rng = StdRng::seed_from_u64(42);

    let stealth_address = {
        let scalar = JubJubScalar::random(&mut rng);
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));

        pk.gen_stealth_address(&scalar)
    };

    let sender = {
        let sender_pk =
            PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
        let note_pk = NotePublicKey::from(&NoteSecretKey::random(&mut rng));
        let blinder = [
            JubJubScalar::random(&mut rng),
            JubJubScalar::random(&mut rng),
        ];

        Sender::encrypt(&note_pk, &sender_pk, &blinder)
    };

    let fee = PhoenixFee {
        gas_limit: rng.next_u64(),
        gas_price: rng.next_u64(),
        stealth_address,
        sender,
    };

    let ser = serde_json::to_string(&fee).unwrap();
    let deser = serde_json::from_str(&ser).unwrap();
    assert_eq!(fee, deser);
}
