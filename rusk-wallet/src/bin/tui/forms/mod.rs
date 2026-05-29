// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod field;

use std::path::{Path, PathBuf};

use rusk_wallet::currency::Dusk;
use rusk_wallet::gas::{
    DEFAULT_LIMIT_CALL, DEFAULT_LIMIT_DEPLOYMENT, DEFAULT_LIMIT_TRANSFER,
    DEFAULT_LIMIT_WALLET_ACTION, DEFAULT_PRICE, MIN_PRICE_DEPLOYMENT,
};
use rusk_wallet::{Address, MAX_FUNCTION_NAME_SIZE, Profile};

use self::field::FormField;
use crate::Command;

const NO_ADDITIONAL_PROFILE_ERROR: &str = "No additional profile created";
const STAKE_OWNER_LOCKED_ERROR: &str =
    "Stake owner is locked by existing stake";

/// Identifiers for the different form types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormId {
    Transfer,
    Stake,
    Unstake,
    ClaimRewards,
    Shield,
    Unshield,
    ContractDeploy,
    ContractCall,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferModel {
    Public,
    Shielded,
    Unknown,
}

/// A form with multiple input fields that can build a Command.
#[derive(Debug, Clone)]
pub struct FormState {
    pub id: FormId,
    pub title: String,
    pub fields: Vec<FormField>,
    pub focused: usize,
    pub profile_idx: u8,
    pub error: Option<String>,
    /// The shielded address for the current profile
    shielded_addr: Address,
    /// The public address for the current profile
    public_addr: Address,
    /// Max spendable shielded balance for transfers.
    transfer_shielded_max: Dusk,
    /// Max spendable public balance for transfers.
    transfer_public_max: Dusk,
    /// Public addresses that can own stake funds.
    stake_owner_addrs: Vec<Address>,
    /// True when an existing stake already fixes the owner key.
    stake_owner_locked: bool,
}

impl FormState {
    pub fn next_field(&mut self) {
        if self.id == FormId::Transfer
            && self
                .fields
                .get(self.focused)
                .is_some_and(|field| field.name == "recipient")
            && self.parse_address("recipient").is_none()
        {
            self.error = Some("Enter a valid recipient address first.".into());
            return;
        }

        if self.focused + 1 < self.fields.len() {
            self.focused += 1;
        }
    }

    pub fn prev_field(&mut self) {
        if self.focused > 0 {
            self.focused -= 1;
        }
    }

    pub fn is_on_last_field(&self) -> bool {
        self.focused + 1 >= self.fields.len()
    }

    pub fn input_char(&mut self, c: char) {
        let is_transfer_recipient = self.id == FormId::Transfer
            && self
                .fields
                .get(self.focused)
                .is_some_and(|field| field.name == "recipient");

        if let Some(field) = self.fields.get_mut(self.focused) {
            field.input_char(c);
        }

        if is_transfer_recipient {
            self.update_transfer_amount_max();
            if self.parse_address("recipient").is_some() {
                self.error = None;
            }
        }
    }

    pub fn delete_char(&mut self) {
        let is_transfer_recipient = self.id == FormId::Transfer
            && self
                .fields
                .get(self.focused)
                .is_some_and(|field| field.name == "recipient");

        if let Some(field) = self.fields.get_mut(self.focused) {
            field.delete_char();
        }

        if is_transfer_recipient {
            self.update_transfer_amount_max();
            if self.parse_address("recipient").is_some() {
                self.error = None;
            }
        }
    }

    pub fn move_cursor_left(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.move_cursor_left();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.move_cursor_right();
        }
    }

    pub fn move_cursor_home(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.move_cursor_home();
        }
    }

    pub fn move_cursor_end(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.move_cursor_end();
        }
    }

    /// Cycle select field to next option.
    pub fn cycle_next(&mut self) {
        if self.focused_stake_owner_is_locked() {
            self.error = Some(STAKE_OWNER_LOCKED_ERROR.into());
            return;
        }

        if self.focused_stake_owner_is_single_profile() {
            self.error = Some(NO_ADDITIONAL_PROFILE_ERROR.into());
            return;
        }

        let updates_stake_amount_max = self.focused_field_is("model");
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.cycle_next();
        }
        if updates_stake_amount_max {
            self.update_stake_amount_max();
        }
    }

    /// Cycle select field to previous option.
    pub fn cycle_prev(&mut self) {
        if self.focused_stake_owner_is_locked() {
            self.error = Some(STAKE_OWNER_LOCKED_ERROR.into());
            return;
        }

        if self.focused_stake_owner_is_single_profile() {
            self.error = Some(NO_ADDITIONAL_PROFILE_ERROR.into());
            return;
        }

        let updates_stake_amount_max = self.focused_field_is("model");
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.cycle_prev();
        }
        if updates_stake_amount_max {
            self.update_stake_amount_max();
        }
    }

    /// Fill max amount for current amount field.
    pub fn set_max(&mut self) {
        if self.id == FormId::Transfer
            && self
                .fields
                .get(self.focused)
                .is_some_and(|field| field.name == "amount")
        {
            let Some(max) = self.transfer_amount_max_for_recipient() else {
                self.error =
                    Some("Enter a valid recipient address first.".into());
                if let Some(idx) = self
                    .fields
                    .iter()
                    .position(|field| field.name == "recipient")
                {
                    self.focused = idx;
                }
                return;
            };

            if let Some(field) = self.fields.get_mut(self.focused) {
                if let field::FieldKind::Amount { max: field_max } =
                    &mut field.kind
                {
                    *field_max = Some(max);
                }
                field.set_max();
            }
            self.error = None;
            return;
        }

        if let Some(field) = self.fields.get_mut(self.focused) {
            field.set_max();
        }
    }

    pub(crate) fn transfer_model(&self) -> TransferModel {
        if self.id != FormId::Transfer {
            return TransferModel::Unknown;
        }

        match self.parse_address("recipient") {
            Some(address) => Self::transfer_model_for_address(&address),
            None => TransferModel::Unknown,
        }
    }

    fn transfer_model_for_address(address: &Address) -> TransferModel {
        match address {
            Address::Public(_) => TransferModel::Public,
            Address::Shielded(_) => TransferModel::Shielded,
        }
    }

    fn transfer_sender_for_address(&self, address: &Address) -> Address {
        match Self::transfer_model_for_address(address) {
            TransferModel::Public => self.public_addr.clone(),
            TransferModel::Shielded => self.shielded_addr.clone(),
            TransferModel::Unknown => unreachable!(
                "a parsed transfer recipient address always resolves to a model"
            ),
        }
    }

    fn transfer_amount_max_for_recipient(&self) -> Option<Dusk> {
        match self.transfer_model() {
            TransferModel::Public => Some(self.transfer_public_max),
            TransferModel::Shielded => Some(self.transfer_shielded_max),
            TransferModel::Unknown => None,
        }
    }

    fn update_transfer_amount_max(&mut self) {
        if self.id != FormId::Transfer {
            return;
        }

        let max = self.transfer_amount_max_for_recipient();
        if let Some(field) =
            self.fields.iter_mut().find(|field| field.name == "amount")
            && let field::FieldKind::Amount { max: field_max } = &mut field.kind
        {
            *field_max = max;
        }
    }

    fn stake_amount_max_for_model(&self) -> Option<Dusk> {
        match self.field_selected("model") {
            Some(0) => Some(self.transfer_shielded_max),
            Some(_) => Some(self.transfer_public_max),
            None => None,
        }
    }

    fn update_stake_amount_max(&mut self) {
        if self.id != FormId::Stake {
            return;
        }

        let max = self.stake_amount_max_for_model();
        if let Some(field) =
            self.fields.iter_mut().find(|field| field.name == "amount")
            && let field::FieldKind::Amount { max: field_max } = &mut field.kind
        {
            *field_max = max;
        }
    }

    /// Returns true if the currently focused field is a select.
    pub fn is_select_field(&self) -> bool {
        self.fields
            .get(self.focused)
            .map(|f| f.is_select())
            .unwrap_or(false)
    }

    /// Returns true if the currently focused field is an amount.
    pub fn is_amount_field(&self) -> bool {
        self.fields
            .get(self.focused)
            .map(|f| matches!(f.kind, field::FieldKind::Amount { .. }))
            .unwrap_or(false)
    }

    fn field_value(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.value.as_str())
    }

    fn field_selected(&self, name: &str) -> Option<usize> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .and_then(|f| f.selected_option)
    }

    fn focused_field_is(&self, name: &str) -> bool {
        self.fields
            .get(self.focused)
            .is_some_and(|field| field.name == name)
    }

    fn focused_stake_owner_is_single_profile(&self) -> bool {
        self.id == FormId::Stake
            && self.focused_field_is("owner")
            && self.stake_owner_addrs.len() <= 1
    }

    fn focused_stake_owner_is_locked(&self) -> bool {
        self.id == FormId::Stake
            && self.focused_field_is("owner")
            && self.stake_owner_locked
    }

    pub fn lock_stake_owner(&mut self, owner: &Address) -> bool {
        if self.id != FormId::Stake {
            return false;
        }

        let Some(owner_idx) =
            self.stake_owner_addrs.iter().position(|addr| addr == owner)
        else {
            return false;
        };

        if let Some(field) =
            self.fields.iter_mut().find(|field| field.name == "owner")
            && let field::FieldKind::Select { options } = &field.kind
            && let Some(option) = options.get(owner_idx)
        {
            field.selected_option = Some(owner_idx);
            field.value = option.clone();
            field.label = "Stake owner (set)".into();
            self.stake_owner_locked = true;
            self.error = None;
            return true;
        }

        false
    }

    /// Try to build a Command from the current form values.
    /// Returns None if validation fails (sets self.error).
    pub fn try_build_command(&mut self) -> Option<Command> {
        self.error = None;
        match self.id {
            FormId::Transfer => self.build_transfer(),
            FormId::Stake => self.build_stake(),
            FormId::Unstake => self.build_unstake(),
            FormId::ClaimRewards => self.build_claim_rewards(),
            FormId::Shield => self.build_shield(),
            FormId::Unshield => self.build_unshield(),
            FormId::ContractDeploy => self.build_contract_deploy(),
            FormId::ContractCall => self.build_contract_call(),
            FormId::Export => self.build_export(),
        }
    }

    fn parse_address(&self, name: &str) -> Option<Address> {
        self.field_value(name)
            .and_then(|v| v.parse::<Address>().ok())
    }

    fn parse_dusk(&self, name: &str) -> Option<Dusk> {
        self.field_value(name)
            .and_then(|v| v.parse::<f64>().ok())
            .and_then(|v| Dusk::try_from(v).ok())
    }

    fn parse_u64(&self, name: &str) -> Option<u64> {
        self.field_value(name).and_then(|v| v.parse::<u64>().ok())
    }

    fn parse_hex(&self, name: &str) -> Option<Vec<u8>> {
        self.field_value(name).and_then(|v| {
            if v.is_empty() {
                Some(Vec::new())
            } else {
                hex::decode(v).ok()
            }
        })
    }

    fn parse_gas(&self, limit_default: u64, price_default: u64) -> (u64, u64) {
        (
            self.parse_u64("gas_limit").unwrap_or(limit_default),
            self.parse_u64("gas_price").unwrap_or(price_default),
        )
    }

    fn require_dusk(&mut self, name: &str, error: &str) -> Option<Dusk> {
        match self.parse_dusk(name) {
            Some(value) => Some(value),
            None => {
                self.error = Some(error.into());
                None
            }
        }
    }

    fn parse_optional_dusk(
        &mut self,
        name: &str,
        error: &str,
    ) -> Option<Option<Dusk>> {
        let is_empty = self.field_value(name).unwrap_or("").trim().is_empty();
        if is_empty {
            return Some(None);
        }

        self.require_dusk(name, error).map(Some)
    }

    fn require_hex(&mut self, name: &str, error: &str) -> Option<Vec<u8>> {
        match self.parse_hex(name) {
            Some(value) => Some(value),
            None => {
                self.error = Some(error.into());
                None
            }
        }
    }

    /// Get the address for the selected transaction model (shielded/public).
    fn model_address(&self) -> Address {
        if self.field_selected("model") == Some(0) {
            self.shielded_addr.clone()
        } else {
            self.public_addr.clone()
        }
    }

    fn build_transfer(&mut self) -> Option<Command> {
        let rcvr = match self.parse_address("recipient") {
            Some(a) => a,
            None => {
                self.error = Some("Invalid recipient address".into());
                return None;
            }
        };

        let sender = self.transfer_sender_for_address(&rcvr);

        let amt = self.require_dusk("amount", "Invalid amount")?;

        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_TRANSFER, DEFAULT_PRICE);
        let memo = self
            .field_value("memo")
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty());

        Some(Command::Transfer {
            sender: Some(sender),
            rcvr,
            amt,
            gas_limit,
            gas_price,
            memo,
        })
    }

    fn build_stake(&mut self) -> Option<Command> {
        let address = Some(self.model_address());

        let amt = self.require_dusk("amount", "Invalid amount")?;

        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_WALLET_ACTION, DEFAULT_PRICE);
        let owner = Some(self.stake_owner_address());

        Some(Command::Stake {
            address,
            owner,
            amt,
            gas_limit,
            gas_price,
        })
    }

    fn build_unstake(&mut self) -> Option<Command> {
        let address = Some(self.model_address());
        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_WALLET_ACTION, DEFAULT_PRICE);

        Some(Command::Unstake {
            address,
            gas_limit,
            gas_price,
        })
    }

    fn build_claim_rewards(&mut self) -> Option<Command> {
        let address = Some(self.model_address());
        let reward =
            self.parse_optional_dusk("amount", "Invalid claim amount")?;
        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_WALLET_ACTION, DEFAULT_PRICE);

        Some(Command::ClaimRewards {
            address,
            reward,
            gas_limit,
            gas_price,
        })
    }

    fn build_shield(&mut self) -> Option<Command> {
        let amt = self.require_dusk("amount", "Invalid amount")?;

        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_WALLET_ACTION, DEFAULT_PRICE);

        Some(Command::Shield {
            profile_idx: Some(self.profile_idx),
            amt,
            gas_limit,
            gas_price,
        })
    }

    fn build_unshield(&mut self) -> Option<Command> {
        let amt = self.require_dusk("amount", "Invalid amount")?;

        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_WALLET_ACTION, DEFAULT_PRICE);

        Some(Command::Unshield {
            profile_idx: Some(self.profile_idx),
            amt,
            gas_limit,
            gas_price,
        })
    }

    fn build_contract_deploy(&mut self) -> Option<Command> {
        let address = Some(self.model_address());

        let code_path = self
            .field_value("code")
            .map(PathBuf::from)
            .unwrap_or_default();
        if code_path.extension().and_then(|e| e.to_str()) != Some("wasm") {
            self.error = Some("Code must be a .wasm file".into());
            return None;
        }

        let init_args =
            self.require_hex("init_args", "Init args must be valid hex")?;
        let deploy_nonce = match self.parse_u64("nonce") {
            Some(n) => n,
            None => {
                self.error = Some("Invalid nonce".into());
                return None;
            }
        };

        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_DEPLOYMENT, MIN_PRICE_DEPLOYMENT);

        Some(Command::ContractDeploy {
            address,
            code: code_path,
            init_args,
            deploy_nonce,
            gas_limit,
            gas_price,
        })
    }

    fn build_contract_call(&mut self) -> Option<Command> {
        let address = Some(self.model_address());

        let contract_id = match self.parse_hex("contract_id") {
            Some(id) if id.len() == 32 => id,
            _ => {
                self.error = Some("Contract ID must be 32 bytes hex".into());
                return None;
            }
        };

        let fn_name =
            self.field_value("fn_name").unwrap_or_default().to_string();
        if fn_name.is_empty() || fn_name.len() > MAX_FUNCTION_NAME_SIZE {
            self.error = Some("Invalid function name".into());
            return None;
        }

        let fn_args =
            self.require_hex("fn_args", "Function args must be valid hex")?;

        let deposit = self
            .parse_optional_dusk("deposit", "Invalid deposit amount")?
            .unwrap_or(Dusk::from(0));
        let (gas_limit, gas_price) =
            self.parse_gas(DEFAULT_LIMIT_CALL, DEFAULT_PRICE);

        Some(Command::ContractCall {
            address,
            contract_id,
            fn_name,
            fn_args,
            gas_limit,
            gas_price,
            deposit,
        })
    }

    fn build_export(&mut self) -> Option<Command> {
        let dir = self
            .field_value("directory")
            .map(PathBuf::from)
            .unwrap_or_default();
        if !dir.is_dir() {
            self.error = Some("Invalid directory".into());
            return None;
        }

        Some(Command::Export {
            profile_idx: Some(self.profile_idx),
            dir,
            name: None,
            export_pwd: None,
        })
    }

    fn stake_owner_address(&self) -> Address {
        let owner_idx = self
            .field_selected("owner")
            .unwrap_or(self.profile_idx as usize);

        self.stake_owner_addrs
            .get(owner_idx)
            .cloned()
            .unwrap_or_else(|| self.public_addr.clone())
    }
}

/// Build a form for the given operation.
pub fn build_form(
    id: FormId,
    profile_idx: u8,
    phoenix_spendable: Dusk,
    moonlight_balance: Dusk,
    claim_rewards_max: Option<Dusk>,
    profiles: &[Profile],
    wallet_dir: &Path,
) -> FormState {
    let profile = &profiles[profile_idx as usize];
    let shielded_addr = Address::Shielded(profile.shielded_addr);
    let public_addr = Address::Public(profile.public_addr);
    let stake_owner_addrs = profiles
        .iter()
        .map(|profile| Address::Public(profile.public_addr))
        .collect();

    let fields = match id {
        FormId::Transfer => vec![
            FormField::text("recipient", "Recipient address"),
            FormField::amount_unknown("amount", "Amount (DUSK)"),
            FormField::text("memo", "Memo (optional)"),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_TRANSFER),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Stake => vec![
            public_model_field(),
            FormField::amount(
                "amount",
                "Stake amount (DUSK)",
                moonlight_balance,
            ),
            stake_owner_field(profiles, profile_idx),
            FormField::number(
                "gas_limit",
                "Gas limit",
                DEFAULT_LIMIT_WALLET_ACTION,
            ),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Unstake => vec![
            public_model_field(),
            FormField::number(
                "gas_limit",
                "Gas limit",
                DEFAULT_LIMIT_WALLET_ACTION,
            ),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::ClaimRewards => vec![
            public_model_field(),
            claim_rewards_amount_field(claim_rewards_max),
            FormField::number(
                "gas_limit",
                "Gas limit",
                DEFAULT_LIMIT_WALLET_ACTION,
            ),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Shield => vec![
            FormField::amount(
                "amount",
                "Amount to shield (DUSK)",
                moonlight_balance,
            ),
            FormField::number(
                "gas_limit",
                "Gas limit",
                DEFAULT_LIMIT_WALLET_ACTION,
            ),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Unshield => vec![
            FormField::amount(
                "amount",
                "Amount to unshield (DUSK)",
                phoenix_spendable,
            ),
            FormField::number(
                "gas_limit",
                "Gas limit",
                DEFAULT_LIMIT_WALLET_ACTION,
            ),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::ContractDeploy => vec![
            public_model_field(),
            FormField::text("code", "WASM contract path"),
            FormField::text("init_args", "Init args (hex)"),
            FormField::number("nonce", "Deploy nonce", 0),
            FormField::number(
                "gas_limit",
                "Gas limit",
                DEFAULT_LIMIT_DEPLOYMENT,
            ),
            FormField::number(
                "gas_price",
                "Gas price (LUX)",
                MIN_PRICE_DEPLOYMENT,
            ),
        ],
        FormId::ContractCall => vec![
            public_model_field(),
            FormField::text("contract_id", "Contract ID (hex)"),
            FormField::text("fn_name", "Function name"),
            FormField::text("fn_args", "Function args (hex)"),
            FormField::amount("deposit", "Deposit (DUSK)", moonlight_balance),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_CALL),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Export => vec![FormField::text_with_default(
            "directory",
            "Export directory",
            &wallet_dir.display().to_string(),
        )],
    };

    let title = match id {
        FormId::Transfer => "Transfer DUSK",
        FormId::Stake => "Stake DUSK",
        FormId::Unstake => "Unstake DUSK",
        FormId::ClaimRewards => "Claim Stake Rewards",
        FormId::Shield => "Shield (Public -> Shielded)",
        FormId::Unshield => "Unshield (Shielded -> Public)",
        FormId::ContractDeploy => "Deploy Contract",
        FormId::ContractCall => "Call Contract",
        FormId::Export => "Export Provisioner Keys",
    };

    FormState {
        id,
        title: title.to_string(),
        fields,
        focused: 0,
        profile_idx,
        error: None,
        shielded_addr,
        public_addr,
        transfer_shielded_max: phoenix_spendable,
        transfer_public_max: moonlight_balance,
        stake_owner_addrs,
        stake_owner_locked: false,
    }
}

fn public_model_field() -> FormField {
    FormField::select_with_default(
        "model",
        "Transaction model",
        vec!["Shielded".into(), "Public".into()],
        1,
    )
}

fn stake_owner_field(profiles: &[Profile], profile_idx: u8) -> FormField {
    let current_idx = profile_idx as usize;
    let options = profiles
        .iter()
        .enumerate()
        .map(|(idx, profile)| {
            let address = Address::Public(profile.public_addr);
            let mut label = format!("Profile {}", idx + 1);
            if idx == current_idx {
                label.push_str(" (current)");
            }
            format!("{label} - {}", address.preview())
        })
        .collect();

    FormField::select_with_default("owner", "Stake owner", options, current_idx)
}

fn claim_rewards_amount_field(max: Option<Dusk>) -> FormField {
    const LABEL: &str = "Amount to claim (DUSK, empty = all)";

    match max {
        Some(max) => FormField::amount("amount", LABEL, max),
        None => FormField::amount_unknown("amount", LABEL),
    }
}

#[cfg(test)]
mod tests {
    use wallet_core::Seed;
    use wallet_core::keys::{derive_bls_pk, derive_phoenix_pk};

    use super::*;

    fn test_profile_at(index: u8) -> Profile {
        let seed: Seed = [7u8; 64];
        Profile {
            shielded_addr: derive_phoenix_pk(&seed, index),
            public_addr: derive_bls_pk(&seed, index),
        }
    }

    fn test_profile() -> Profile {
        test_profile_at(0)
    }

    fn transfer_form() -> FormState {
        let temp_dir = std::env::temp_dir();

        build_form(
            FormId::Transfer,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &[test_profile()],
            temp_dir.as_path(),
        )
    }

    fn stake_form() -> FormState {
        let temp_dir = std::env::temp_dir();

        build_form(
            FormId::Stake,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &[test_profile()],
            temp_dir.as_path(),
        )
    }

    fn stake_form_with_profiles(
        profile_idx: u8,
        profiles: &[Profile],
    ) -> FormState {
        let temp_dir = std::env::temp_dir();

        build_form(
            FormId::Stake,
            profile_idx,
            Dusk::from(11),
            Dusk::from(22),
            None,
            profiles,
            temp_dir.as_path(),
        )
    }

    fn set_recipient(form: &mut FormState, address: &Address) {
        let field = form
            .fields
            .iter_mut()
            .find(|field| field.name == "recipient")
            .expect("transfer form should have a recipient field");
        field.value = address.to_string();
        field.cursor = field.value.len();
    }

    fn amount_max(form: &FormState) -> Option<Dusk> {
        let field = form
            .fields
            .iter()
            .find(|field| field.name == "amount")
            .expect("transfer form should have an amount field");
        match field.kind {
            field::FieldKind::Amount { max } => max,
            _ => panic!("amount field should use amount kind"),
        }
    }

    fn focus_field(form: &mut FormState, name: &str) {
        form.focused = form
            .fields
            .iter()
            .position(|field| field.name == name)
            .expect("form should have requested field");
    }

    fn set_amount(form: &mut FormState, amount: &str) {
        let field = form
            .fields
            .iter_mut()
            .find(|field| field.name == "amount")
            .expect("form should have an amount field");
        field.value = amount.to_string();
        field.cursor = field.value.len();
    }

    #[test]
    fn public_recipient_resolves_to_public_transfer_model() {
        let mut form = transfer_form();
        let profile = test_profile();
        set_recipient(&mut form, &Address::Public(profile.public_addr));

        assert_eq!(form.transfer_model(), TransferModel::Public);
    }

    #[test]
    fn shielded_recipient_resolves_to_shielded_transfer_model() {
        let mut form = transfer_form();
        let profile = test_profile();
        set_recipient(&mut form, &Address::Shielded(profile.shielded_addr));

        assert_eq!(form.transfer_model(), TransferModel::Shielded);
    }

    #[test]
    fn invalid_recipient_resolves_to_unknown_transfer_model() {
        let mut form = transfer_form();
        let field = form
            .fields
            .iter_mut()
            .find(|field| field.name == "recipient")
            .expect("transfer form should have a recipient field");
        field.value = "not-an-address".into();
        field.cursor = field.value.len();

        assert_eq!(form.transfer_model(), TransferModel::Unknown);
    }

    #[test]
    fn transfer_amount_max_tracks_derived_transfer_model() {
        let profile = test_profile();
        let mut form = transfer_form();

        assert_eq!(amount_max(&form), None);

        set_recipient(&mut form, &Address::Public(profile.public_addr));
        form.update_transfer_amount_max();
        assert_eq!(amount_max(&form), Some(Dusk::from(22)));

        set_recipient(&mut form, &Address::Shielded(profile.shielded_addr));
        form.update_transfer_amount_max();
        assert_eq!(amount_max(&form), Some(Dusk::from(11)));

        let field = form
            .fields
            .iter_mut()
            .find(|field| field.name == "recipient")
            .expect("transfer form should have a recipient field");
        field.value = "invalid".into();
        field.cursor = field.value.len();
        form.update_transfer_amount_max();
        assert_eq!(amount_max(&form), None);
    }

    #[test]
    fn stake_amount_max_tracks_selected_transaction_model() {
        let mut form = stake_form();

        assert_eq!(amount_max(&form), Some(Dusk::from(22)));

        form.cycle_prev();
        assert_eq!(amount_max(&form), Some(Dusk::from(11)));

        form.cycle_next();
        assert_eq!(amount_max(&form), Some(Dusk::from(22)));
    }

    #[test]
    fn stake_can_select_different_owner_profile() {
        let profiles = vec![test_profile_at(0), test_profile_at(1)];
        let mut form = stake_form_with_profiles(0, &profiles);

        form.cycle_prev();
        focus_field(&mut form, "owner");
        form.cycle_next();
        set_amount(&mut form, "1");

        let command = form.try_build_command().expect("valid stake command");
        let Command::Stake { address, owner, .. } = command else {
            panic!("expected stake command");
        };

        assert_eq!(address, Some(Address::Shielded(profiles[0].shielded_addr)));
        assert_eq!(owner, Some(Address::Public(profiles[1].public_addr)));
    }

    #[test]
    fn locked_stake_owner_cannot_be_changed() {
        let profiles = vec![test_profile_at(0), test_profile_at(1)];
        let mut form = stake_form_with_profiles(0, &profiles);
        let locked_owner = Address::Public(profiles[1].public_addr);

        assert!(form.lock_stake_owner(&locked_owner));
        focus_field(&mut form, "owner");

        form.cycle_next();

        let owner_field = form
            .fields
            .iter()
            .find(|field| field.name == "owner")
            .expect("stake form should have owner field");
        assert_eq!(owner_field.selected_option, Some(1));
        assert_eq!(
            form.error.as_deref(),
            Some("Stake owner is locked by existing stake")
        );

        form.error = None;
        set_amount(&mut form, "1");

        let command = form.try_build_command().expect("valid stake command");
        let Command::Stake { owner, .. } = command else {
            panic!("expected stake command");
        };

        assert_eq!(owner, Some(locked_owner));
    }
}
