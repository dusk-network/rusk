// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use canonical::{Canon, InvalidEncoding, Store};
use canonical_derive::Canon;
use dusk_abi::{ContractId, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::PublicKey;
use phoenix_core::{Crossover, Fee, Message, Note};

#[derive(Debug, Clone, Canon)]
pub enum Call {
    Execute {
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        fee: Fee,
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        spend_proof: Vec<u8>,
        call: Option<(ContractId, Transaction)>,
    },

    SendToContractTransparent {
        address: BlsScalar,
        value: u64,
        spend_proof: Vec<u8>,
    },

    WithdrawFromTransparent {
        address: BlsScalar,
        note: Note,
    },

    SendToContractObfuscated {
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        spend_proof: Vec<u8>,
    },

    WithdrawFromObfuscated {
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    },

    WithdrawFromTransparentToContract {
        from: BlsScalar,
        to: BlsScalar,
        value: u64,
    },
}

impl Call {
    pub fn execute(
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        fee: Fee,
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        spend_proof: Vec<u8>,
        call: Option<(ContractId, Transaction)>,
    ) -> Self {
        Self::Execute {
            anchor,
            nullifiers,
            fee,
            crossover,
            notes,
            spend_proof,
            call,
        }
    }

    pub fn to_execute<S>(
        &self,
        contract: ContractId,
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        fee: Fee,
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        spend_proof: Vec<u8>,
    ) -> Result<Self, S::Error>
    where
        S: Store,
    {
        // Prevents invalid recursion
        if let Self::Execute { .. } = self {
            Err(InvalidEncoding.into())?;
        }

        let tx = Transaction::from_canon(self, &S::default())?;
        let execute = Self::execute(
            anchor,
            nullifiers,
            fee,
            crossover,
            notes,
            spend_proof,
            Some((contract, tx)),
        );

        Ok(execute)
    }

    pub fn send_to_contract_transparent(
        address: BlsScalar,
        value: u64,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::SendToContractTransparent {
            address,
            value,
            spend_proof,
        }
    }

    pub fn withdraw_from_transparent(address: BlsScalar, note: Note) -> Self {
        Self::WithdrawFromTransparent { address, note }
    }

    pub fn send_to_contract_obfuscated(
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::SendToContractObfuscated {
            address,
            message,
            r,
            pk,
            spend_proof,
        }
    }

    pub fn withdraw_from_obfuscated(
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::WithdrawFromObfuscated {
            address,
            message,
            r,
            pk,
            note,
            input_value_commitment,
            spend_proof,
        }
    }

    pub fn withdraw_from_transparent_to_contract(
        from: BlsScalar,
        to: BlsScalar,
        value: u64,
    ) -> Self {
        Self::WithdrawFromTransparentToContract { from, to, value }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use crate::TransferContract;

    impl Call {
        pub fn transact<S>(self, contract: &mut TransferContract<S>) -> bool
        where
            S: Store,
        {
            match self {
                Call::Execute {
                    anchor,
                    nullifiers,
                    fee,
                    crossover,
                    notes,
                    spend_proof,
                    call,
                } => contract.execute(
                    anchor,
                    nullifiers,
                    fee,
                    crossover,
                    notes,
                    spend_proof,
                    call,
                ),

                Call::SendToContractTransparent {
                    address,
                    value,
                    spend_proof,
                } => contract.send_to_contract_transparent(
                    address,
                    value,
                    spend_proof,
                ),

                Call::WithdrawFromTransparent { address, note } => {
                    contract.withdraw_from_transparent(address, note)
                }

                Call::SendToContractObfuscated {
                    address,
                    message,
                    r,
                    pk,
                    spend_proof,
                } => contract.send_to_contract_obfuscated(
                    address,
                    message,
                    r,
                    pk,
                    spend_proof,
                ),

                Call::WithdrawFromObfuscated {
                    address,
                    message,
                    r,
                    pk,
                    note,
                    input_value_commitment,
                    spend_proof,
                } => contract.withdraw_from_obfuscated(
                    address,
                    message,
                    r,
                    pk,
                    note,
                    input_value_commitment,
                    spend_proof,
                ),

                Call::WithdrawFromTransparentToContract { from, to, value } => {
                    contract
                        .withdraw_from_transparent_to_contract(from, to, value)
                }
            }
        }
    }
}
