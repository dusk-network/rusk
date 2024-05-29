// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeMap;
use core::ops::{Deref, DerefMut};

use bls12_381_bls::PublicKey as BlsPublicKey;
use dusk_bytes::Serializable as _;
use moonlight_contract_types::{
    Account, Deposit, MoonlightEvent, Transfer, Withdraw,
};
use rusk_abi::TRANSFER_CONTRACT;
use transfer_contract_types::{Stct, WfctRaw};

/// The Moonlight contract maintains a mapping of an address to account states.
///
/// Comprised of a nonce and balance, the account state is utilized to monitor
/// and update transaction counters during balance adjustments (transfers,
/// withdrawals), while deposits leave the nonce unchanged and merely enhance
/// the account's balance.
#[derive(Debug, Default, Clone)]
pub struct MoonlightState {
    accounts: BTreeMap<[u8; BlsPublicKey::SIZE], Account>,
}

impl Deref for MoonlightState {
    type Target = BTreeMap<[u8; BlsPublicKey::SIZE], Account>;

    fn deref(&self) -> &Self::Target {
        &self.accounts
    }
}

impl DerefMut for MoonlightState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.accounts
    }
}

impl MoonlightState {
    /// Creates a new empty instance of the accounts set.
    pub const fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
        }
    }

    /// Deposits a note into the account, consuming it via the `send to contract
    /// transparent` functionality of the transfer contract.
    ///
    /// This operation will not affect the nonce of the account that receives
    /// the funds.
    pub fn deposit(&mut self, deposit: Deposit) {
        let message = transfer.to_signature_message();
        if !rusk_abi::verify_bls(
            message,
            transfer.from_address,
            transfer.signature,
        ) {
            panic!("Invalid signature!");
        }

        let account = self.get_account_mut(&deposit.address);

        account.balance = account
            .balance
            .checked_add(deposit.value)
            .expect("account balance overflow");

        let stct = Stct {
            module: rusk_abi::self_id().to_bytes(),
            value: deposit.value,
            proof: deposit.proof,
        };

        let res = rusk_abi::call(TRANSFER_CONTRACT, "stct", &stct)
            .expect("failed to consume note");

        assert!(res, "failed to consume note");

        rusk_abi::emit(
            "deposit",
            MoonlightEvent {
                active_address: None,
                passive_address: Some(deposit.address),
                value: deposit.value,
            },
        );
    }

    /// Transfer a balance from and into the provided accounts.
    ///
    /// This function will assert and mutate the nonce of the active account.
    pub fn transfer(&mut self, transfer: Transfer) {
        let message = transfer.to_signature_message();
        if !rusk_abi::verify_bls(
            message,
            transfer.from_address,
            transfer.signature,
        ) {
            panic!("Invalid signature!");
        }

        {
            let from_account = self.get_account_mut(&transfer.from_address);

            assert_eq!(from_account.nonce, transfer.nonce);

            from_account.nonce = from_account.nonce.wrapping_add(1);
            from_account.balance = from_account
                .balance
                .checked_sub(transfer.value)
                .expect("insufficient balance");
        }

        {
            let to_account = self.get_account_mut(&transfer.to_address);

            to_account.balance = to_account
                .balance
                .checked_add(transfer.value)
                .expect("balance overflow");
        }

        rusk_abi::emit(
            "transfer",
            MoonlightEvent {
                active_address: Some(transfer.from_address),
                passive_address: Some(transfer.to_address),
                value: transfer.value,
            },
        );
    }

    /// Deducts the balance from the account and generates a new Phoenix note
    /// via "withdraw from contract transparent". a note into the account,
    /// consuming it via the `send to contract transparent`
    ///
    /// This function will assert and mutate the nonce of the active account.
    pub fn withdraw(&mut self, withdraw: Withdraw) {
        let message = withdraw.to_signature_message();
        if !rusk_abi::verify_bls(message, withdraw.address, withdraw.signature)
        {
            panic!("Invalid signature!");
        }

        let account = self.get_account_mut(&withdraw.address);

        assert_eq!(account.nonce, withdraw.nonce);

        account.nonce = account.nonce.wrapping_add(1);
        account.balance = account
            .balance
            .checked_sub(withdraw.value)
            .expect("insufficient balance");

        let res = rusk_abi::call(
            TRANSFER_CONTRACT,
            "wfct_raw",
            &WfctRaw {
                value: withdraw.value,
                note: withdraw.note,
                proof: withdraw.proof,
            },
        )
        .expect("failed to withdraw note");

        assert!(res, "failed to withdraw note");

        rusk_abi::emit(
            "withdraw",
            MoonlightEvent {
                active_address: Some(withdraw.address),
                passive_address: None,
                value: withdraw.value,
            },
        );
    }

    /// Gets or creates a default instance of the account state mapped by the
    /// provided key.
    pub(crate) fn get_account_mut(
        &mut self,
        address: &BlsPublicKey,
    ) -> &mut Account {
        self.accounts.entry(address.to_bytes()).or_default()
    }
}
