// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, *};

use dusk_abi::{ContractId, Transaction};
use dusk_bls12_381_sign::{PublicKey, Signature};
use dusk_bytes::Serializable;
use dusk_pki::StealthAddress;
use dusk_schnorr::Signature as SchnorrSignature;
use lazy_static::lazy_static;
use phoenix_core::{Crossover, Fee, Note};
use transfer_circuits::{
    SendToContractTransparentCircuit, WithdrawFromTransparentCircuit,
};

use dusk_plonk::prelude::*;

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

fn prove<C>(mut circuit: C) -> Result<Vec<u8>, Error>
where
    C: Circuit,
{
    let pk = rusk_profile::keys_for(&C::CIRCUIT_ID)
        .map_err(|_| Error::PlonkKeys)
        .and_then(|keys| keys.get_prover().map_err(|_| Error::PlonkKeys))
        .and_then(|pk| {
            ProverKey::from_slice(pk.as_slice()).map_err(|_| Error::PlonkKeys)
        })?;

    let proof = circuit
        .prove(&PP, &pk, rusk_abi::TRANSCRIPT_LABEL)
        .map_err(|_| Error::PlonkProver)?
        .to_bytes()
        .to_vec();

    Ok(proof)
}

impl StakeContract {
    pub fn stake_transaction(
        fee: &Fee,
        crossover: &Crossover,
        blinder: JubJubScalar,
        stct_signature: SchnorrSignature,
        pk: PublicKey,
        signature: Signature,
        value: u64,
    ) -> Result<(ContractId, Transaction), Error> {
        let id = rusk_abi::stake_contract();
        let address = rusk_abi::contract_to_scalar(&id);

        let circuit = SendToContractTransparentCircuit::new(
            fee,
            crossover,
            value,
            blinder,
            address,
            stct_signature,
        );

        let proof = prove(circuit)?;

        let transaction = (TX_STAKE, pk, signature, value, proof);
        let transaction = Transaction::from_canon(&transaction);

        Ok((id, transaction))
    }

    pub fn unstake_transaction(
        pk: PublicKey,
        signature: Signature,
        note: Note,
        blinder: JubJubScalar,
    ) -> Result<(ContractId, Transaction), Error> {
        let id = rusk_abi::stake_contract();

        let value = note.value(None).expect("Transparent note");
        let commitment = *note.value_commitment();

        let circuit =
            WithdrawFromTransparentCircuit::new(commitment, value, blinder);

        let proof = prove(circuit)?;

        let transaction = (TX_UNSTAKE, pk, signature, note, proof);
        let transaction = Transaction::from_canon(&transaction);

        Ok((id, transaction))
    }

    pub fn withdraw_transaction(
        pk: PublicKey,
        signature: Signature,
        address: StealthAddress,
        nonce: BlsScalar,
    ) -> (ContractId, Transaction) {
        let id = rusk_abi::stake_contract();

        let transaction = (TX_WITHDRAW, pk, signature, address, nonce);
        let transaction = Transaction::from_canon(&transaction);

        (id, transaction)
    }

    pub fn allowlist_transaction(
        pk: PublicKey,
        signature: Signature,
        owner: PublicKey,
    ) -> (ContractId, Transaction) {
        let id = rusk_abi::stake_contract();

        let transaction = (TX_ADD_ALLOWLIST, pk, owner, signature);
        let transaction = Transaction::from_canon(&transaction);

        (id, transaction)
    }
}
