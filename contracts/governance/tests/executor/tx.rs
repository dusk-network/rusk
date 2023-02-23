// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Sink};
use dusk_abi::Transaction;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_pki::PublicKey;
use governance_contract::{
    Transfer, TX_FEE, TX_MINT, TX_PAUSE, TX_TRANSFER, TX_UNPAUSE,
};

use std::sync::atomic::{AtomicU64, Ordering};

static SEED: AtomicU64 = AtomicU64::new(0);

pub fn seed() -> BlsScalar {
    BlsScalar::from(SEED.fetch_add(1, Ordering::SeqCst))
}

fn signed_transaction<C>(sk: &BlsSecretKey, payload: C) -> Transaction
where
    C: Canon,
{
    let payload_len = payload.encoded_len();
    let capacity = payload_len + (payload_len as u32).encoded_len();
    let len_u32 = capacity as u32;
    let mut buffer = vec![0; capacity];

    let mut sink = Sink::new(&mut buffer);
    len_u32.encode(&mut sink);
    payload.encode(&mut sink);

    let pk = BlsPublicKey::from(sk);
    let signature = sk.sign(&pk, &buffer);

    let transaction = (signature, len_u32, payload);
    Transaction::from_canon(&transaction)
}

pub fn transfer<T>(
    sk_authority: &BlsSecretKey,
    seed: BlsScalar,
    transfers: Vec<T>,
) -> Transaction
where
    T: Into<Transfer>,
{
    let transfers: Vec<Transfer> =
        transfers.into_iter().map(|t| t.into()).collect();

    signed_transaction(sk_authority, (seed, TX_TRANSFER, transfers))
}

pub fn fee<T>(
    sk_authority: &BlsSecretKey,
    seed: BlsScalar,
    transfers: Vec<T>,
) -> Transaction
where
    T: Into<Transfer>,
{
    let transfers: Vec<Transfer> =
        transfers.into_iter().map(|t| t.into()).collect();

    signed_transaction(sk_authority, (seed, TX_FEE, transfers))
}

pub fn mint(
    sk_authority: &BlsSecretKey,
    seed: BlsScalar,
    address: &PublicKey,
    amount: u64,
) -> Transaction {
    signed_transaction(sk_authority, (seed, TX_MINT, *address, amount))
}

pub fn pause(sk_authority: &BlsSecretKey, seed: BlsScalar) -> Transaction {
    signed_transaction(sk_authority, (seed, TX_PAUSE))
}

pub fn unpause(sk_authority: &BlsSecretKey, seed: BlsScalar) -> Transaction {
    signed_transaction(sk_authority, (seed, TX_UNPAUSE))
}
