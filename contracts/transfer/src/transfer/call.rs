// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use canonical::{ByteSource, Canon, InvalidEncoding, Store};
use canonical_derive::Canon;
use dusk_abi::{ContractId, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::PublicKey;
use phoenix_core::{Crossover, Fee, Message, Note};

#[derive(Debug, Clone, Canon)]
pub enum Call {
    External {
        contract: ContractId,
        transaction: Transaction,
    },

    SendToContractTransparent {
        address: BlsScalar,
        value: u64,
        pk: PublicKey,
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
        crossover_pk: PublicKey,
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
    // Can be trivially converted to `Into` once issue #71 is solved
    fn to_transaction<S>(&self) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        // FIXME BytesSink should not require `store`
        // https://github.com/dusk-network/canonical/issues/71
        let store: &S =
            unsafe { (&() as *const ()).cast::<S>().as_ref().unwrap() };

        Transaction::from_canon(self, store)
    }

    pub fn external<S>(
        contract: ContractId,
        transaction: Transaction,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::External {
            contract,
            transaction,
        };

        call.to_transaction::<S>()
    }

    pub fn send_to_contract_transparent<S>(
        address: BlsScalar,
        value: u64,
        pk: PublicKey,
        spend_proof: Vec<u8>,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::SendToContractTransparent {
            address,
            value,
            pk,
            spend_proof,
        };

        call.to_transaction::<S>()
    }

    pub fn withdraw_from_transparent<S>(
        address: BlsScalar,
        note: Note,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::WithdrawFromTransparent { address, note };

        call.to_transaction::<S>()
    }

    pub fn send_to_contract_obfuscated<S>(
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        crossover_pk: PublicKey,
        spend_proof: Vec<u8>,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::SendToContractObfuscated {
            address,
            message,
            r,
            pk,
            crossover_pk,
            spend_proof,
        };

        call.to_transaction::<S>()
    }

    pub fn withdraw_from_obfuscated<S>(
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::WithdrawFromObfuscated {
            address,
            message,
            r,
            pk,
            note,
            input_value_commitment,
            spend_proof,
        };

        call.to_transaction::<S>()
    }

    pub fn withdraw_from_transparent_to_contract<S>(
        from: BlsScalar,
        to: BlsScalar,
        value: u64,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::WithdrawFromTransparentToContract { from, to, value };

        call.to_transaction::<S>()
    }
}

#[derive(Debug, Clone, Canon)]
pub enum InternalCall {
    None(Option<Crossover>),

    External {
        contract: ContractId,
        transaction: Transaction,
        crossover: Option<Crossover>,
    },

    SendToContractTransparent {
        address: BlsScalar,
        value: u64,
        crossover: Crossover,
        pk: PublicKey,
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
        crossover: Crossover,
        crossover_pk: PublicKey,
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

#[derive(Debug, Clone, Canon)]
pub struct TransferExecute {
    pub(crate) anchor: BlsScalar,
    pub(crate) nullifiers: Vec<BlsScalar>,
    pub(crate) fee: Fee,
    pub(crate) crossover: Option<Crossover>,
    pub(crate) notes: Vec<Note>,
    pub(crate) spend_proof: Vec<u8>,
    pub(crate) call: Option<Transaction>,
}

impl TransferExecute {
    pub fn new(
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        fee: Fee,
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        spend_proof: Vec<u8>,
        call: Option<Transaction>,
    ) -> Self {
        Self {
            anchor,
            nullifiers,
            fee,
            crossover,
            notes,
            spend_proof,
            call,
        }
    }

    pub fn into_internal<S: Store>(self) -> Result<InternalCall, S::Error> {
        let TransferExecute {
            crossover, call, ..
        } = self;

        let call: Option<Call> = match call {
            Some(tx) => {
                // FIXME Transact should implement `cast` by its own.
                // https://github.com/dusk-network/rusk-vm/issues/158
                //
                // FIXME BytesSource should not require `store`
                // https://github.com/dusk-network/canonical/issues/71
                let store: &S =
                    unsafe { (&() as *const ()).cast::<S>().as_ref().unwrap() };

                let mut source = ByteSource::new(tx.as_bytes(), store);

                Some(Canon::<S>::read(&mut source)?)
            }

            None => None,
        };

        let call = match (crossover, call) {
            (
                crossover,
                Some(Call::External {
                    contract,
                    transaction,
                }),
            ) => InternalCall::External {
                contract,
                transaction,
                crossover,
            },

            (
                Some(crossover),
                Some(Call::SendToContractTransparent {
                    address,
                    value,
                    pk,
                    spend_proof,
                }),
            ) => InternalCall::SendToContractTransparent {
                address,
                value,
                crossover,
                pk,
                spend_proof,
            },

            (None, Some(Call::WithdrawFromTransparent { address, note })) => {
                InternalCall::WithdrawFromTransparent { address, note }
            }

            (
                Some(crossover),
                Some(Call::SendToContractObfuscated {
                    address,
                    message,
                    r,
                    pk,
                    crossover_pk,
                    spend_proof,
                }),
            ) => InternalCall::SendToContractObfuscated {
                address,
                message,
                r,
                pk,
                crossover,
                crossover_pk,
                spend_proof,
            },

            (
                None,
                Some(Call::WithdrawFromObfuscated {
                    address,
                    message,
                    r,
                    pk,
                    note,
                    input_value_commitment,
                    spend_proof,
                }),
            ) => InternalCall::WithdrawFromObfuscated {
                address,
                message,
                r,
                pk,
                note,
                input_value_commitment,
                spend_proof,
            },

            (
                None,
                Some(Call::WithdrawFromTransparentToContract {
                    from,
                    to,
                    value,
                }),
            ) => InternalCall::WithdrawFromTransparentToContract {
                from,
                to,
                value,
            },

            (_, None) => InternalCall::None(crossover),

            _ => return Err(InvalidEncoding.into()),
        };

        Ok(call)
    }
}

#[derive(Debug, Clone)]
pub struct InternalCallResult {
    pub status: bool,
    pub crossover: Option<Crossover>,
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    impl InternalCallResult {
        pub const fn error() -> Self {
            Self {
                status: false,
                crossover: None,
            }
        }

        pub const fn success(crossover: Option<Crossover>) -> Self {
            Self {
                status: true,
                crossover,
            }
        }

        pub const fn is_success(&self) -> bool {
            self.status
        }
    }
}
