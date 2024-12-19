// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;
use alloc::string::String;
use bytecheck::CheckBytes;
use dusk_bytes::Serializable;
use dusk_core::abi::{self, ContractId};
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, Signature as BlsSignature,
};
use dusk_core::transfer::ReceiveFromContract;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct OwnerMessage {
    contract_id: ContractId,
    args: u8,
    fname: String,
    nonce: u64,
}

/// Bob contract.
#[derive(Debug, Clone)]
pub struct Bob {
    value: u8,
    nonce: u64,
    total_dusk: u64,
}

impl Bob {
    pub const fn new() -> Self {
        Self {
            value: 0,
            nonce: 0,
            total_dusk: 0,
        }
    }

    #[allow(dead_code)]
    pub fn identifier() -> &'static [u8; 3] {
        b"bob"
    }
}

impl Bob {
    pub fn init(&mut self, n: u8) {
        self.value = n;
        self.nonce = 0;
    }

    pub fn reset(&mut self, n: u8) {
        self.value = n;
    }

    pub fn owner_reset(&mut self, sig: BlsSignature, msg: OwnerMessage) {
        let mut granted = false;
        let message_bytes = rkyv::to_bytes::<_, 4096>(&msg)
            .expect("Message should serialize correctly")
            .to_vec();

        let owner_bytes = abi::self_owner_raw();
        if let Ok(owner) = BlsPublicKey::from_bytes(&owner_bytes) {
            if self.nonce == msg.nonce
                && msg.fname == "owner_reset"
                && msg.contract_id == abi::self_id()
                && abi::verify_bls(message_bytes, owner, sig)
            {
                self.owner_only_function(msg.args);
                self.nonce += 1;
                granted = true;
            }
        }
        if !granted {
            panic!("method restricted only to the owner")
        }
    }

    fn owner_only_function(&mut self, args: u8) {
        self.value = args;
    }

    pub fn ping(&mut self) {}

    pub fn echo(&mut self, n: u64) -> u64 {
        n
    }

    pub fn value(&mut self) -> u8 {
        self.value
    }

    pub fn nonce(&mut self) -> u64 {
        self.nonce
    }

    pub fn recv_transfer(&mut self, recv: ReceiveFromContract) {
        self.total_dusk += recv.value;
    }
}
