// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use dusk_bytes::Serializable;
use execution_core::transfer::Stct;
use execution_core::{BlsPublicKey, BlsSignature};
use rkyv::{Archive, Deserialize, Serialize};
use rusk_abi::TRANSFER_CONTRACT;

#[derive(Debug, Clone)]
pub struct Charlie;

/// Subsidy a contract with a value.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Subsidy {
    /// Public key to which the subsidy will belong.
    pub public_key: BlsPublicKey,
    /// Signature belonging to the given public key.
    pub signature: BlsSignature,
    /// Value of the subsidy.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Vec<u8>,
}

const SUBSIDY_MESSAGE_SIZE: usize = u64::SIZE + u64::SIZE;

/// Return the digest to be signed in the `subsidize` function of a contract.
#[must_use]
pub fn subsidy_signature_message(
    counter: u64,
    value: u64,
) -> [u8; SUBSIDY_MESSAGE_SIZE] {
    let mut bytes = [0u8; SUBSIDY_MESSAGE_SIZE];

    bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
    bytes[u64::SIZE..].copy_from_slice(&value.to_bytes());

    bytes
}

impl Charlie {
    fn gas_price() -> u64 {
        rusk_abi::call::<(), u64>(TRANSFER_CONTRACT, "gas_price", &())
            .expect("Obtaining gas price should succeed")
    }

    /// calling this method will be paid by the contract
    pub fn pay(&mut self) {
        const ALLOWANCE: u64 = 60_000_000;
        let allowance = ALLOWANCE / Self::gas_price();
        // this call is paid for by the contract, up to 'allowance'
        rusk_abi::set_allowance(allowance);
    }

    /// calling this method should be paid by the contract, yet it
    /// sets the allowance to a value too small to cover
    /// the execution cost, transaction will fail
    /// and contract balance won't be affected
    pub fn pay_and_fail(&mut self) {
        const ALLOWANCE: u64 = 80_000;
        let allowance = ALLOWANCE / Self::gas_price();
        // this call is paid for by the contract, up to 'allowance'
        rusk_abi::set_allowance(allowance);
    }

    /// this method calls the `pay` method indirectly, and in such case, since
    /// allowance is set by an indirectly called method, it won't have effect
    /// and contract balance won't be affected
    pub fn pay_indirectly_and_fail(&mut self) {
        rusk_abi::call::<_, ()>(rusk_abi::self_id(), "pay", &())
            .expect("pay call should succeed");
    }

    /// Subsidizes the contract with funds which can then be used
    /// for sponsoring free uses of other methods of this contract.
    /// Funds passed in this call will be used when granting allowances.
    /// The subsidy operation is similar to staking, yet the funds
    /// are deposited in this contract's "wallet".
    pub fn subsidize(&mut self, subsidy: Subsidy) {
        // verify the signature is over the correct digest
        // note: counter is always zero - make sure that this is safe
        let digest = subsidy_signature_message(0, subsidy.value).to_vec();

        if !rusk_abi::verify_bls(digest, subsidy.public_key, subsidy.signature)
        {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let transfer_module = TRANSFER_CONTRACT;

        let stct = Stct {
            module: rusk_abi::self_id().to_bytes(),
            value: subsidy.value,
            proof: subsidy.proof,
        };

        // subsidizing self with 'subsidy.value'
        rusk_abi::call::<_, bool>(transfer_module, "stct", &stct)
            .expect("Sending note to contract should succeed");
    }
}
