// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::convert::TryFrom;

use alloc::vec::Vec;
use canonical::Canon;
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::PublicKey;
use phoenix_core::{Crossover, Fee, Message, Note};

#[derive(Debug, Clone, Canon)]
pub enum Call {
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

#[derive(Debug, Clone, Canon)]
pub struct TransferExecute {
    pub anchor: BlsScalar,
    pub nullifiers: Vec<BlsScalar>,
    pub fee: Fee,
    pub crossover: Option<Crossover>,
    pub notes: Vec<Note>,
    pub spend_proof: Vec<u8>,
    pub call: Option<Call>,
}

#[derive(Debug, Clone, Canon)]
pub enum InternalCall {
    None(Option<Crossover>),

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

impl TryFrom<TransferExecute> for InternalCall {
    // TODO Use a concrete error definition
    type Error = ();

    fn try_from(execute: TransferExecute) -> Result<Self, Self::Error> {
        let TransferExecute {
            crossover, call, ..
        } = execute;

        let call = match (crossover, call) {
            (
                Some(crossover),
                Some(Call::SendToContractTransparent {
                    address,
                    value,
                    pk,
                    spend_proof,
                }),
            ) => Self::SendToContractTransparent {
                address,
                value,
                crossover,
                pk,
                spend_proof,
            },

            (None, Some(Call::WithdrawFromTransparent { address, note })) => {
                Self::WithdrawFromTransparent { address, note }
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
            ) => Self::SendToContractObfuscated {
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
            ) => Self::WithdrawFromObfuscated {
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
            ) => Self::WithdrawFromTransparentToContract { from, to, value },

            (_, None) => Self::None(crossover),

            _ => return Err(()),
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
