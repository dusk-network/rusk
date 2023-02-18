// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake2::{digest::consts::U32, Digest};
use canonical::EncodeToVec;
use dusk_abi::Transaction;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    Signature as BlsSignature,
};
use dusk_bytes::Serializable;
use dusk_pki::PublicKey;
use governance_contract::{Transfer, TX_MINT, TX_TRANSFER};
use std::iter;

type Blake2b = blake2::Blake2b<U32>;

pub fn transfer(
    transfers: Vec<Transfer>,
    sk_authority: &BlsSecretKey,
) -> Transaction {
    let seed = seed(&transfers);

    let scalars = iter::once([seed, BlsScalar::from(TX_TRANSFER as u64)])
        .flatten()
        .chain(transfers.iter().flat_map(Transfer::as_scalars))
        .collect::<Vec<_>>();

    let signature = sign(sk_authority, &scalars);

    let transaction = (TX_TRANSFER, seed, signature, transfers);
    Transaction::from_canon(&transaction)
}

pub fn mint(
    address: &PublicKey,
    amount: u64,
    sk_authority: &BlsSecretKey,
) -> Transaction {
    let mut r = rand::thread_rng();
    let seed = BlsScalar::random(&mut r);
    let scalars = iter::once([seed, BlsScalar::from(TX_MINT as u64)])
        .chain(iter::once(address.as_ref().to_hash_inputs()))
        .flatten()
        .chain(iter::once(BlsScalar::from(amount)))
        .collect::<Vec<_>>();

    let signature = sign(sk_authority, &scalars);

    let transaction = (TX_MINT, seed, signature, *address, amount);
    Transaction::from_canon(&transaction)
}

fn sign(sk: &BlsSecretKey, scalars: &[BlsScalar]) -> BlsSignature {
    let scalar_bytes = &dusk_poseidon::sponge::hash(scalars).to_bytes();

    let pk = BlsPublicKey::from(sk);
    sk.sign(&pk, &scalar_bytes[..])
}

fn seed(data: &Vec<Transfer>) -> BlsScalar {
    let msg = data.encode_to_vec();
    let mut digest: [u8; BlsScalar::SIZE] = Blake2b::digest(msg).into();

    // Truncate the contract id to fit bls
    digest[31] &= 0x3f;

    BlsScalar::from_bytes(&digest).unwrap_or_default()
}
