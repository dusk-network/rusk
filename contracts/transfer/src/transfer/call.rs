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
use dusk_pki::StealthAddress;
use phoenix_core::{Crossover, Fee, Message, Note};

#[allow(clippy::large_enum_variant)]
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
        message_address: StealthAddress,
        spend_proof: Vec<u8>,
    },

    WithdrawFromObfuscated {
        message: Message,
        message_address: StealthAddress,
        change: Message,
        change_address: StealthAddress,
        output: Note,
        spend_proof: Vec<u8>,
    },

    WithdrawFromTransparentToContract {
        to: ContractId,
        value: u64,
    },

    Mint {
        notes: Vec<Note>,
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

    pub fn to_transaction(&self) -> Transaction {
        Transaction::from_canon(self)
    }

    #[allow(clippy::too_many_arguments)]
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
            return Err(Error::ExecuteRecursion);
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
        message_address: StealthAddress,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::SendToContractObfuscated {
            address,
            message,
            message_address,
            spend_proof,
        }
    }

    pub fn withdraw_from_obfuscated(
        message: Message,
        message_address: StealthAddress,
        change: Message,
        change_address: StealthAddress,
        output: Note,
        spend_proof: Vec<u8>,
    ) -> Self {
        Self::WithdrawFromObfuscated {
            message,
            message_address,
            change,
            change_address,
            output,
            spend_proof,
        }
    }

    pub fn withdraw_from_transparent_to_contract(
        to: ContractId,
        value: u64,
    ) -> Self {
        Self::WithdrawFromTransparentToContract { to, value }
    }

    pub fn mint(notes: Vec<Note>) -> Self {
        Self::Mint { notes }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use crate::TransferContract;

    use dusk_abi::ReturnValue;

    impl Call {
        pub fn transact(self, contract: &mut TransferContract) -> ReturnValue {
            match self {
                Call::Execute {
                    anchor,
                    nullifiers,
                    fee,
                    crossover,
                    notes,
                    spend_proof,
                    call,
                } => ReturnValue::from_canon(&contract.execute(
                    anchor,
                    nullifiers,
                    fee,
                    crossover,
                    notes,
                    spend_proof,
                    call,
                )),

                Call::SendToContractTransparent {
                    address,
                    value,
                    spend_proof,
                } => ReturnValue::from_canon(
                    &contract.send_to_contract_transparent(
                        address,
                        value,
                        spend_proof,
                    ),
                ),

                Call::WithdrawFromTransparent {
                    value,
                    note,
                    spend_proof,
                } => ReturnValue::from_canon(
                    &contract.withdraw_from_transparent(
                        value,
                        note,
                        spend_proof,
                    ),
                ),

                Call::SendToContractObfuscated {
                    address,
                    message,
                    message_address,
                    spend_proof,
                } => ReturnValue::from_canon(
                    &contract.send_to_contract_obfuscated(
                        address,
                        message,
                        message_address,
                        spend_proof,
                    ),
                ),

                Call::WithdrawFromObfuscated {
                    message,
                    message_address,
                    change,
                    change_address,
                    output,
                    spend_proof,
                } => {
                    ReturnValue::from_canon(&contract.withdraw_from_obfuscated(
                        message,
                        message_address,
                        change,
                        change_address,
                        output,
                        spend_proof,
                    ))
                }

                Call::WithdrawFromTransparentToContract { to, value } => {
                    ReturnValue::from_canon(
                        &contract
                            .withdraw_from_transparent_to_contract(to, value),
                    )
                }

                Call::Mint { notes } => {
                    ReturnValue::from_canon(&contract.mint(notes))
                }
            }
        }
    }
}
