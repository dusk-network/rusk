// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Error;

use alloc::vec::Vec;
use canonical::Canon;
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
        address: ContractId,
        value: u64,
        spend_proof: Vec<u8>,
    },

    WithdrawFromTransparent {
        value: u64,
        note: Note,
        spend_proof: Vec<u8>,
    },

    SendToContractObfuscated {
        address: ContractId,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        spend_proof: Vec<u8>,
    },

    WithdrawFromObfuscated {
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    },

    WithdrawFromTransparentToContract {
        to: ContractId,
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

    pub fn to_execute(
        &self,
        contract: ContractId,
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        fee: Fee,
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        spend_proof: Vec<u8>,
    ) -> Result<Self, Error> {
        if let Self::Execute { .. } = self {
            Err(Error::ExecuteRecursion)?;
        }

        let tx = Transaction::from_canon(self);
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
        address: ContractId,
        value: u64,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::SendToContractTransparent {
            address,
            value,
            spend_proof,
        }
    }

    pub fn withdraw_from_transparent(
        value: u64,
        note: Note,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::WithdrawFromTransparent {
            value,
            note,
            spend_proof,
        }
    }

    pub fn send_to_contract_obfuscated(
        address: ContractId,
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
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::WithdrawFromObfuscated {
            message,
            r,
            pk,
            note,
            input_value_commitment,
            spend_proof,
        }
    }

    pub fn withdraw_from_transparent_to_contract(
        to: ContractId,
        value: u64,
    ) -> Self {
        Self::WithdrawFromTransparentToContract { to, value }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use crate::TransferContract;

    impl Call {
        pub fn transact(self, contract: &mut TransferContract) -> bool {
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

                Call::WithdrawFromTransparent {
                    value,
                    note,
                    spend_proof,
                } => {
                    contract.withdraw_from_transparent(value, note, spend_proof)
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
                    message,
                    r,
                    pk,
                    note,
                    input_value_commitment,
                    spend_proof,
                } => contract.withdraw_from_obfuscated(
                    message,
                    r,
                    pk,
                    note,
                    input_value_commitment,
                    spend_proof,
                ),

                Call::WithdrawFromTransparentToContract { to, value } => {
                    contract.withdraw_from_transparent_to_contract(to, value)
                }
            }
        }
    }
}
