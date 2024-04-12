// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_abi::TRANSFER_CONTRACT;
use subsidy_types::*;
use transfer_contract_types::Stct;

#[derive(Debug, Clone)]
pub struct Charlie;

impl Charlie {
    pub fn pay(&mut self) {
        const ALLOWANCE: u64 = 10_000_000;
        rusk_abi::debug!(
            "CHARLIE pay - this call is free up to {} of allowance",
            ALLOWANCE
        );
        rusk_abi::set_allowance(ALLOWANCE);
    }

    /// we set charge to a value which is too small to cover
    /// execution cost, the transaction should fail
    /// and contract balance should not be affected
    pub fn pay_and_fail(&mut self) {
        const ALLOWANCE: u64 = 4_000_000;
        rusk_abi::debug!(
            "CHARLIE pay - this call is free up to {} of allowance",
            ALLOWANCE
        );
        rusk_abi::set_allowance(ALLOWANCE);
    }

    pub fn earn(&mut self) {
        const CHARGE: u64 = 20_000_000;
        rusk_abi::debug!("CHARLIE earn - charging {} for the call", CHARGE);
        rusk_abi::set_charge(CHARGE);
    }

    /// we set charge to a value which is too small to cover
    /// execution cost, the transaction should fail
    /// and contract balance should not be affected
    pub fn earn_and_fail(&mut self) {
        const CHARGE: u64 = 4_000_000;
        rusk_abi::debug!("CHARLIE earn - charging {} for the call", CHARGE);
        rusk_abi::set_charge(CHARGE);
    }

    /// loads the contract with funds which can be used
    /// to sponsor free uses of some methods of this contract
    /// technically, the funds passed in this call will be used
    /// when granting allowances
    /// this operation is similar to staking, but the funds
    /// are staked into this contract's "wallet" rather than
    /// into the stake contract's wallet
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

        rusk_abi::debug!(
            "CHARLIE subsidize - subsidized self ('{:X?}') with value {}",
            stct.module[0],
            stct.value
        );

        rusk_abi::call::<_, bool>(transfer_module, "stct", &stct)
            .expect("Sending note to contract should succeed");
    }
}
