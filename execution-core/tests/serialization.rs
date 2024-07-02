// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_bytes::{Error, Serializable};
use dusk_jubjub::JubJubScalar;
use execution_core::transfer::{ContractCall, Fee, Payload, Transaction};
use execution_core::{Note, PublicKey, SecretKey, TxSkeleton};
use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn transaction_from_to_bytes() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    // set the general parameters
    let sender_pk = PublicKey::from(&SecretKey::random(&mut rng));
    let receiver_pk = PublicKey::from(&SecretKey::random(&mut rng));

    let gas_limit = 500;
    let gas_price = 42;

    // build the tx-skeleton
    let value = 25;
    let value_blinder = JubJubScalar::random(&mut rng);
    let sender_blinder = [
        JubJubScalar::random(&mut rng),
        JubJubScalar::random(&mut rng),
    ];
    let note = Note::obfuscated(
        &mut rng,
        &sender_pk,
        &receiver_pk,
        value,
        value_blinder,
        sender_blinder,
    );

    let root = BlsScalar::from(123);
    let nullifiers = vec![
        BlsScalar::from(456),
        BlsScalar::from(789),
        BlsScalar::from(6583),
        BlsScalar::from(98978542),
    ];
    let outputs = [note.clone(), note];
    let max_fee = gas_limit * gas_price;
    let deposit = 10;

    let tx_skeleton = TxSkeleton {
        root,
        nullifiers,
        outputs,
        max_fee,
        deposit,
    };

    // build the fee
    let fee = Fee::new(&mut rng, &sender_pk, gas_limit, gas_price);

    // build the contract-call
    let contract = [42; 32];
    let call = ContractCall {
        contract,
        fn_name: String::from("deposit"),
        fn_args: deposit.to_bytes().to_vec(),
    };

    // build the payload
    let payload = Payload::new(tx_skeleton, fee, Some(call));

    // set a random proof
    let proof = [42; 42].to_vec();

    let transaction = Transaction::new(payload, proof);

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;
    assert_eq!(transaction, deserialized);
    Ok(())
}
