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
    DEFAULT_PRICE, MIN_PRICE_DEPLOYMENT,
};
use rusk_wallet::{Address, MAX_FUNCTION_NAME_SIZE, Profile};

use self::field::FormField;
use crate::Command;

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
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.cycle_next();
        }
    }

    /// Cycle select field to previous option.
    pub fn cycle_prev(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.cycle_prev();
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
                    *field_max = max;
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

    fn transfer_amount_max_for_recipient(&self) -> Option<Dusk> {
        match self.parse_address("recipient") {
            Some(Address::Public(_)) => Some(self.transfer_public_max),
            Some(Address::Shielded(_)) => Some(self.transfer_shielded_max),
            None => None,
        }
    }

    fn update_transfer_amount_max(&mut self) {
        if self.id != FormId::Transfer {
            return;
        }

        let Some(max) = self.transfer_amount_max_for_recipient() else {
            return;
        };

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

        // Match sender to recipient address type
        let sender = match &rcvr {
            Address::Shielded(_) => self.shielded_addr.clone(),
            Address::Public(_) => self.public_addr.clone(),
        };

        let amt = match self.parse_dusk("amount") {
            Some(a) => a,
            None => {
                self.error = Some("Invalid amount".into());
                return None;
            }
        };

        let gas_limit = self
            .parse_u64("gas_limit")
            .unwrap_or(DEFAULT_LIMIT_TRANSFER);
        let gas_price = self.parse_u64("gas_price").unwrap_or(DEFAULT_PRICE);
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

        let amt = match self.parse_dusk("amount") {
            Some(a) => a,
            None => {
                self.error = Some("Invalid amount".into());
                return None;
            }
        };

        let gas_limit =
            self.parse_u64("gas_limit").unwrap_or(DEFAULT_LIMIT_CALL);
        let gas_price = self.parse_u64("gas_price").unwrap_or(DEFAULT_PRICE);
        let owner = Some(self.public_addr.clone());

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
        let gas_limit =
            self.parse_u64("gas_limit").unwrap_or(DEFAULT_LIMIT_CALL);
        let gas_price = self.parse_u64("gas_price").unwrap_or(DEFAULT_PRICE);

        Some(Command::Unstake {
            address,
            gas_limit,
            gas_price,
        })
    }

    fn build_claim_rewards(&mut self) -> Option<Command> {
        let address = Some(self.model_address());
        let reward_raw = self.field_value("amount").unwrap_or("").trim();
        let reward = if reward_raw.is_empty() {
            None
        } else {
            match self.parse_dusk("amount") {
                Some(r) => Some(r),
                None => {
                    self.error = Some("Invalid claim amount".into());
                    return None;
                }
            }
        };
        let gas_limit =
            self.parse_u64("gas_limit").unwrap_or(DEFAULT_LIMIT_CALL);
        let gas_price = self.parse_u64("gas_price").unwrap_or(DEFAULT_PRICE);

        Some(Command::ClaimRewards {
            address,
            reward,
            gas_limit,
            gas_price,
        })
    }

    fn build_shield(&mut self) -> Option<Command> {
        let amt = match self.parse_dusk("amount") {
            Some(a) => a,
            None => {
                self.error = Some("Invalid amount".into());
                return None;
            }
        };

        let gas_limit =
            self.parse_u64("gas_limit").unwrap_or(DEFAULT_LIMIT_CALL);
        let gas_price = self.parse_u64("gas_price").unwrap_or(DEFAULT_PRICE);

        Some(Command::Shield {
            profile_idx: Some(self.profile_idx),
            amt,
            gas_limit,
            gas_price,
        })
    }

    fn build_unshield(&mut self) -> Option<Command> {
        let amt = match self.parse_dusk("amount") {
            Some(a) => a,
            None => {
                self.error = Some("Invalid amount".into());
                return None;
            }
        };

        let gas_limit =
            self.parse_u64("gas_limit").unwrap_or(DEFAULT_LIMIT_CALL);
        let gas_price = self.parse_u64("gas_price").unwrap_or(DEFAULT_PRICE);

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

        let init_raw = self.field_value("init_args").unwrap_or("").trim();
        let init_args = if init_raw.is_empty() {
            Vec::new()
        } else {
            match self.parse_hex("init_args") {
                Some(v) => v,
                None => {
                    self.error = Some("Init args must be valid hex".into());
                    return None;
                }
            }
        };
        let deploy_nonce = match self.parse_u64("nonce") {
            Some(n) => n,
            None => {
                self.error = Some("Invalid nonce".into());
                return None;
            }
        };

        let gas_limit = self
            .parse_u64("gas_limit")
            .unwrap_or(DEFAULT_LIMIT_DEPLOYMENT);
        let gas_price =
            self.parse_u64("gas_price").unwrap_or(MIN_PRICE_DEPLOYMENT);

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

        let args_raw = self.field_value("fn_args").unwrap_or("").trim();
        let fn_args = if args_raw.is_empty() {
            Vec::new()
        } else {
            match self.parse_hex("fn_args") {
                Some(v) => v,
                None => {
                    self.error = Some("Function args must be valid hex".into());
                    return None;
                }
            }
        };

        let deposit_raw = self.field_value("deposit").unwrap_or("").trim();
        let deposit = if deposit_raw.is_empty() {
            Dusk::from(0)
        } else {
            match self.parse_dusk("deposit") {
                Some(d) => d,
                None => {
                    self.error = Some("Invalid deposit amount".into());
                    return None;
                }
            }
        };
        let gas_limit =
            self.parse_u64("gas_limit").unwrap_or(DEFAULT_LIMIT_CALL);
        let gas_price = self.parse_u64("gas_price").unwrap_or(DEFAULT_PRICE);

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
}

/// Build a form for the given operation.
pub fn build_form(
    id: FormId,
    profile_idx: u8,
    phoenix_spendable: Dusk,
    moonlight_balance: Dusk,
    profiles: &[Profile],
    wallet_dir: &Path,
) -> FormState {
    let profile = &profiles[profile_idx as usize];
    let shielded_addr = Address::Shielded(profile.shielded_addr);
    let public_addr = Address::Public(profile.public_addr);

    let fields = match id {
        FormId::Transfer => vec![
            FormField::text("recipient", "Recipient address"),
            FormField::amount("amount", "Amount (DUSK)", phoenix_spendable),
            FormField::text("memo", "Memo (optional)"),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_TRANSFER),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Stake => vec![
            // Default to Public — stake amount max is moonlight_balance
            FormField::select_with_default(
                "model",
                "Transaction model",
                vec!["Shielded".into(), "Public".into()],
                1,
            ),
            FormField::amount(
                "amount",
                "Stake amount (DUSK)",
                moonlight_balance,
            ),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_CALL),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Unstake => vec![
            FormField::select(
                "model",
                "Transaction model",
                vec!["Shielded".into(), "Public".into()],
            ),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_CALL),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::ClaimRewards => vec![
            FormField::select(
                "model",
                "Transaction model",
                vec!["Shielded".into(), "Public".into()],
            ),
            FormField::amount(
                "amount",
                "Amount to claim (DUSK, empty = all)",
                moonlight_balance,
            ),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_CALL),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Shield => vec![
            FormField::amount(
                "amount",
                "Amount to shield (DUSK)",
                moonlight_balance,
            ),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_CALL),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::Unshield => vec![
            FormField::amount(
                "amount",
                "Amount to unshield (DUSK)",
                phoenix_spendable,
            ),
            FormField::number("gas_limit", "Gas limit", DEFAULT_LIMIT_CALL),
            FormField::number("gas_price", "Gas price (LUX)", DEFAULT_PRICE),
        ],
        FormId::ContractDeploy => vec![
            FormField::select(
                "model",
                "Transaction model",
                vec!["Shielded".into(), "Public".into()],
            ),
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
            FormField::select(
                "model",
                "Transaction model",
                vec!["Shielded".into(), "Public".into()],
            ),
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
    }
}
