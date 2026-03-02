// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dusk_core::stake::StakeData;
use rusk_wallet::currency::Dusk;
use rusk_wallet::{Address, MAX_PROFILES, Profile, Wallet};
use wallet_core::BalanceInfo;

use crate::command::TransactionHistory;
use crate::settings::Settings;
use crate::{Command, WalletFile};

use super::action::AsyncResult;
use super::forms::{self, FormId, FormState};

/// Cached balance data for a profile.
#[derive(Debug, Clone, Default)]
pub struct ProfileBalance {
    pub phoenix: Option<BalanceInfo>,
    pub moonlight: Option<Dusk>,
}

/// Connection status to Rusk services.
#[derive(Debug, Clone, Default)]
pub struct ConnectionStatus {
    pub state: bool,
}

/// Cached stake data for a profile.
#[derive(Debug, Clone)]
pub enum StakeState {
    /// Fetched successfully with data.
    Loaded(StakeData),
    /// Fetched successfully but no stake exists.
    NoStake,
}

/// Sync state tracking.
#[derive(Debug, Clone, Default)]
pub enum SyncStatus {
    #[default]
    Unknown,
    Syncing,
    Synced,
    Error(String),
}

/// Result info for display after a command completes.
#[derive(Debug, Clone)]
pub enum ResultInfo {
    TxSent {
        tx_hash: String,
        explorer_url: Option<String>,
    },
    DeployTxSent {
        tx_hash: String,
        contract_id: String,
        explorer_url: Option<String>,
    },
    ExportedKeys {
        pub_key_path: String,
        key_pair_path: String,
    },
    Error {
        message: String,
    },
}

/// The currently active screen in the TUI.
#[derive(Debug)]
pub enum AppScreen {
    /// Main dashboard with profile info and actions
    Dashboard,
    /// A form overlay on top of the dashboard
    Form { form: Box<FormState> },
    /// Confirmation dialog before executing a command
    Confirmation,
    /// Command is executing (loading state)
    Executing { description: String },
    /// Result display after command execution
    Result { info: ResultInfo },
    /// Transaction history browser (replaces menu area)
    History { entries: Vec<TransactionHistory> },
    /// Stake info display (replaces menu area)
    StakeInfo,
    /// Help overlay showing keybindings
    Help,
}

/// Dashboard menu items. Order matches the menu list.
/// The "p" entry label is overridden dynamically by `App::menu_label()`.
pub const MENU_ITEMS: &[(&str, &str)] = &[
    ("t", "Transfer DUSK"),
    ("s", "Stake DUSK"),
    ("u", "Unstake"),
    ("c", "Claim Rewards"),
    ("h", "Transaction History"),
    ("i", "Stake Info"),
    ("d", "Shield (Public \u{2192} Shielded)"),
    ("e", "Unshield (Shielded \u{2192} Public)"),
    ("x", "Deploy Contract"),
    ("X", "Call Contract"),
    ("k", "Export Keys"),
    ("m", "Import Wallet"),
    ("p", "Switch Profile"),
    ("a", "Add Profile"),
    ("?", "Help"),
    ("q", "Quit"),
];

/// Central application state.
pub struct App<'a> {
    pub wallet: &'a mut Wallet<WalletFile>,
    pub settings: &'a Settings,

    // Navigation
    pub screen: AppScreen,
    pub profile_idx: u8,
    pub menu_selected: usize,

    // Cached data
    pub balances: HashMap<u8, ProfileBalance>,
    pub stake_info: HashMap<u8, StakeState>,
    pub sync_status: SyncStatus,
    pub connection: ConnectionStatus,

    // Pending command for confirmation flow
    pub pending_cmd: Option<Command>,
    pub pending_cmd_description: Vec<String>,

    // Status messages from wallet operations
    pub status_messages: Vec<String>,

    // History scroll state
    pub history_selected: usize,

    // Last submitted form (for retry on error)
    pub last_form: Option<Box<FormState>>,

    // Control
    pub should_quit: bool,
}

impl<'a> App<'a> {
    pub fn new(
        wallet: &'a mut Wallet<WalletFile>,
        settings: &'a Settings,
    ) -> Self {
        Self {
            wallet,
            settings,
            screen: AppScreen::Dashboard,
            profile_idx: 0,
            menu_selected: 0,
            balances: HashMap::new(),
            stake_info: HashMap::new(),
            sync_status: SyncStatus::default(),
            connection: ConnectionStatus::default(),
            pending_cmd: None,
            pending_cmd_description: Vec::new(),
            status_messages: Vec::new(),
            history_selected: 0,
            last_form: None,
            should_quit: false,
        }
    }

    /// Handle a keyboard event. Returns true if a wallet operation should be
    /// triggered.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<AppAction> {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('c')
        {
            self.should_quit = true;
            return None;
        }

        match &mut self.screen {
            AppScreen::Dashboard => self.handle_dashboard_key(key),
            AppScreen::Form { form } => {
                Self::handle_form_key(form, key, &self.settings.explorer)
            }
            AppScreen::Confirmation => self.handle_confirmation_key(key),
            AppScreen::Executing { .. } => {
                // Only Ctrl+C works during execution (handled above)
                None
            }
            AppScreen::Result { .. } => {
                self.handle_result_key(key);
                None
            }
            AppScreen::History { .. } | AppScreen::StakeInfo => {
                self.handle_history_key(key);
                None
            }
            AppScreen::Help => {
                // Any key closes help
                self.screen = AppScreen::Dashboard;
                None
            }
        }
    }

    fn handle_dashboard_key(&mut self, key: KeyEvent) -> Option<AppAction> {
        match key.code {
            // Arrow / vim navigation through menu
            KeyCode::Up | KeyCode::Char('k') => {
                if self.menu_selected == 0 {
                    self.menu_selected = MENU_ITEMS.len() - 1;
                } else {
                    self.menu_selected -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.menu_selected =
                    (self.menu_selected + 1) % MENU_ITEMS.len();
                None
            }
            // Enter activates the selected menu item
            KeyCode::Enter => {
                let (key, _) = MENU_ITEMS[self.menu_selected];
                self.activate_menu_item(key)
            }
            KeyCode::Char(c) => {
                let key_str = c.to_string();
                // Find and select the matching menu item
                if let Some(pos) =
                    MENU_ITEMS.iter().position(|(k, _)| *k == key_str)
                {
                    self.menu_selected = pos;
                    self.activate_menu_item(&key_str)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Activate a menu item by its hotkey string.
    fn activate_menu_item(&mut self, key: &str) -> Option<AppAction> {
        match key {
            "q" => {
                self.should_quit = true;
                None
            }
            "?" => {
                self.screen = AppScreen::Help;
                None
            }
            "p" => {
                let num_profiles = self.wallet.profiles().len() as u8;
                if num_profiles > 1 {
                    self.profile_idx = (self.profile_idx + 1) % num_profiles;
                    return Some(AppAction::RefreshBalance);
                }
                None
            }
            "a" => {
                if self.wallet.profiles().len() < MAX_PROFILES {
                    let idx = self.wallet.add_profile();
                    let _ = self.wallet.save();
                    self.profile_idx = idx;
                    return Some(AppAction::RefreshBalance);
                }
                None
            }
            "t" => {
                self.open_form(FormId::Transfer);
                None
            }
            "s" => {
                self.open_form(FormId::Stake);
                None
            }
            "u" => {
                self.open_form(FormId::Unstake);
                None
            }
            "c" => {
                self.open_form(FormId::ClaimRewards);
                None
            }
            "d" => {
                self.open_form(FormId::Shield);
                None
            }
            "e" => {
                self.open_form(FormId::Unshield);
                None
            }
            "h" => Some(AppAction::FetchHistory),
            "x" => {
                self.open_form(FormId::ContractDeploy);
                None
            }
            "X" => {
                self.open_form(FormId::ContractCall);
                None
            }
            "k" => {
                self.open_form(FormId::Export);
                None
            }
            "i" => Some(AppAction::FetchStakeInfo),
            "m" => Some(AppAction::ImportWallet),
            _ => None,
        }
    }

    fn handle_form_key(
        form: &mut FormState,
        key: KeyEvent,
        _explorer: &Option<url::Url>,
    ) -> Option<AppAction> {
        match key.code {
            KeyCode::Esc => {
                return Some(AppAction::CloseForm);
            }
            KeyCode::Tab | KeyCode::Down => {
                form.next_field();
            }
            KeyCode::BackTab | KeyCode::Up => {
                form.prev_field();
            }
            KeyCode::Enter => {
                if form.is_on_last_field() {
                    if let Some(cmd) = form.try_build_command() {
                        return Some(AppAction::ConfirmCommand(cmd));
                    }
                } else {
                    form.next_field();
                }
            }
            KeyCode::Char(c) => {
                // 'm'/'M' fills max for amount fields
                if matches!(c, 'm' | 'M') && form.is_amount_field() {
                    form.set_max();
                } else {
                    form.input_char(c);
                }
            }
            KeyCode::Backspace => {
                form.delete_char();
            }
            KeyCode::Left => {
                if form.is_select_field() {
                    form.cycle_prev();
                } else {
                    form.move_cursor_left();
                }
            }
            KeyCode::Right => {
                if form.is_select_field() {
                    form.cycle_next();
                } else {
                    form.move_cursor_right();
                }
            }
            KeyCode::Home => {
                form.move_cursor_home();
            }
            KeyCode::End => {
                form.move_cursor_end();
            }
            _ => {}
        }
        None
    }

    fn handle_confirmation_key(&mut self, key: KeyEvent) -> Option<AppAction> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if let Some(cmd) = self.pending_cmd.take() {
                    let desc = self.pending_cmd_description.join(", ");
                    self.screen = AppScreen::Executing { description: desc };
                    self.status_messages.clear();
                    return Some(AppAction::ExecuteCommand(cmd));
                }
                None
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.pending_cmd = None;
                self.pending_cmd_description.clear();
                self.screen = AppScreen::Dashboard;
                None
            }
            _ => None,
        }
    }

    fn handle_result_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = AppScreen::Dashboard;
            }
            KeyCode::Char('r') => {
                // Retry: re-open the last form if this was an error
                if matches!(
                    &self.screen,
                    AppScreen::Result {
                        info: ResultInfo::Error { .. }
                    }
                ) {
                    if let Some(form) = self.last_form.take() {
                        self.screen = AppScreen::Form { form };
                        return;
                    }
                }
                self.screen = AppScreen::Dashboard;
            }
            KeyCode::Char('o') => {
                // Open explorer if available
                if let AppScreen::Result {
                    info:
                        ResultInfo::TxSent {
                            explorer_url: Some(url),
                            ..
                        }
                        | ResultInfo::DeployTxSent {
                            explorer_url: Some(url),
                            ..
                        },
                } = &self.screen
                {
                    let _ = open::that(url);
                }
                self.screen = AppScreen::Dashboard;
            }
            _ => {}
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = AppScreen::Dashboard;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.history_selected > 0 {
                    self.history_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let AppScreen::History { entries } = &self.screen {
                    if self.history_selected + 1 < entries.len() {
                        self.history_selected += 1;
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle an async result from wallet operations.
    pub fn handle_async_result(&mut self, result: AsyncResult) {
        match result {
            AsyncResult::BalanceUpdate {
                profile_idx,
                phoenix,
                moonlight,
            } => {
                let entry = self.balances.entry(profile_idx).or_default();
                if let Some(p) = phoenix {
                    entry.phoenix = Some(p);
                }
                if let Some(m) = moonlight {
                    entry.moonlight = Some(m);
                }
            }
            AsyncResult::StakeUpdate { profile_idx, stake } => {
                self.stake_info.insert(profile_idx, stake);
            }
            AsyncResult::SyncStatus(msg) | AsyncResult::StatusMessage(msg) => {
                if msg.contains("Complete") || msg.contains("complete") {
                    self.sync_status = SyncStatus::Synced;
                } else if msg.contains("Error") || msg.contains("error") {
                    self.sync_status = SyncStatus::Error(msg.clone());
                } else if msg.contains("Syncing") || msg.contains("syncing") {
                    self.sync_status = SyncStatus::Syncing;
                }
                self.push_status(msg);
            }
            AsyncResult::TxComplete(hash) => {
                let tx_hash = hex::encode(hash.to_bytes());
                let explorer_url = self
                    .settings
                    .explorer
                    .as_ref()
                    .map(|e| format!("{}{}", e, tx_hash));
                self.screen = AppScreen::Result {
                    info: ResultInfo::TxSent {
                        tx_hash,
                        explorer_url,
                    },
                };
            }
            AsyncResult::DeployTxComplete(hash, contract_id) => {
                let tx_hash = hex::encode(hash.to_bytes());
                let cid = hex::encode(contract_id.as_bytes());
                let explorer_url = self
                    .settings
                    .explorer
                    .as_ref()
                    .map(|e| format!("{}{}", e, tx_hash));
                self.screen = AppScreen::Result {
                    info: ResultInfo::DeployTxSent {
                        tx_hash,
                        contract_id: cid,
                        explorer_url,
                    },
                };
            }
            AsyncResult::HistoryFetched(entries) => {
                self.history_selected = 0;
                self.screen = AppScreen::History { entries };
            }
            AsyncResult::ExportedKeys(pub_key, key_pair) => {
                self.screen = AppScreen::Result {
                    info: ResultInfo::ExportedKeys {
                        pub_key_path: pub_key.display().to_string(),
                        key_pair_path: key_pair.display().to_string(),
                    },
                };
            }
            AsyncResult::Error(msg) => {
                self.screen = AppScreen::Result {
                    info: ResultInfo::Error { message: msg },
                };
            }
        }
    }

    fn push_status(&mut self, msg: String) {
        self.status_messages.push(msg);
        // Keep last 5 messages
        if self.status_messages.len() > 5 {
            self.status_messages.remove(0);
        }
    }

    fn open_form(&mut self, form_id: FormId) {
        let _profile = self.current_profile();
        let bal = self.current_balance();
        let phoenix_spendable = bal
            .phoenix
            .as_ref()
            .map(|b| Dusk::from(b.spendable))
            .unwrap_or(Dusk::from(0));
        let moonlight_bal = bal.moonlight.unwrap_or(Dusk::from(0));

        let form = forms::build_form(
            form_id,
            self.profile_idx,
            phoenix_spendable,
            moonlight_bal,
            self.wallet.profiles(),
            &self.settings.wallet_dir,
        );

        self.screen = AppScreen::Form {
            form: Box::new(form),
        };
    }

    pub fn num_profiles(&self) -> usize {
        self.wallet.profiles().len()
    }

    pub fn current_profile(&self) -> &Profile {
        &self.wallet.profiles()[self.profile_idx as usize]
    }

    pub fn current_balance(&self) -> ProfileBalance {
        self.balances
            .get(&self.profile_idx)
            .cloned()
            .unwrap_or_default()
    }

    /// Build confirmation details for a command.
    pub fn prepare_confirmation(&mut self, cmd: Command) {
        // Save the current form for retry on error
        if let AppScreen::Form { form } = &self.screen {
            self.last_form = Some(form.clone());
        }
        let details = build_confirmation_details(&cmd, self.wallet);
        self.pending_cmd_description = details;
        self.pending_cmd = Some(cmd);
        self.screen = AppScreen::Confirmation;
    }
}

/// Actions the TUI event loop should take after handling input.
pub enum AppAction {
    RefreshBalance,
    FetchHistory,
    FetchStakeInfo,
    ImportWallet,
    CloseForm,
    ConfirmCommand(Command),
    ExecuteCommand(Command),
}

/// Build human-readable confirmation details for a command.
fn build_confirmation_details(
    cmd: &Command,
    _wallet: &Wallet<WalletFile>,
) -> Vec<String> {
    let mut d = Vec::new();
    let fee =
        |gl: &u64, gp: &u64| format!("Max fee: {} DUSK", Dusk::from(gl * gp));
    let payer = |a: &Address| format!("Pay with: {}", a.preview());

    match cmd {
        Command::Transfer {
            sender,
            rcvr,
            amt,
            gas_limit,
            gas_price,
            memo,
        } => {
            if let Some(s) = sender {
                d.push(payer(s));
            }
            d.push(format!("Recipient: {}", rcvr.preview()));
            d.push(format!("Amount: {amt} DUSK"));
            if matches!(memo, Some(m) if !m.is_empty()) {
                d.push(format!("Memo: {}", memo.as_deref().unwrap()));
            }
            d.push(fee(gas_limit, gas_price));
            if matches!(sender, Some(Address::Public(_))) {
                d.push("PUBLIC TRANSACTION".into());
            }
        }
        Command::Stake {
            address,
            owner,
            amt,
            gas_limit,
            gas_price,
        } => {
            if let Some(a) = address {
                d.push(payer(a));
            }
            if let Some(o) = owner {
                d.push(format!("Stake owner: {}", o.preview()));
            }
            d.push(format!("Amount: {amt} DUSK"));
            d.push(fee(gas_limit, gas_price));
        }
        Command::Unstake {
            address,
            gas_limit,
            gas_price,
        } => {
            if let Some(a) = address {
                d.push(payer(a));
            }
            d.push(fee(gas_limit, gas_price));
        }
        Command::ClaimRewards {
            address,
            reward,
            gas_limit,
            gas_price,
        } => {
            if let Some(a) = address {
                d.push(payer(a));
            }
            d.push(match reward {
                Some(r) => format!("Claiming: {r} DUSK"),
                None => "Claiming: all rewards".into(),
            });
            d.push(fee(gas_limit, gas_price));
        }
        Command::Shield {
            amt,
            gas_limit,
            gas_price,
            ..
        } => {
            d.push(format!("Shield amount: {amt} DUSK"));
            d.push(fee(gas_limit, gas_price));
        }
        Command::Unshield {
            amt,
            gas_limit,
            gas_price,
            ..
        } => {
            d.push(format!("Unshield amount: {amt} DUSK"));
            d.push(fee(gas_limit, gas_price));
        }
        Command::ContractDeploy {
            address,
            code,
            deploy_nonce,
            gas_limit,
            gas_price,
            ..
        } => {
            if let Some(a) = address {
                d.push(payer(a));
            }
            d.push(format!("Code: {code:?}"));
            d.push(format!("Nonce: {deploy_nonce}"));
            d.push(fee(gas_limit, gas_price));
        }
        Command::ContractCall {
            address,
            contract_id,
            fn_name,
            gas_limit,
            gas_price,
            deposit,
            ..
        } => {
            if let Some(a) = address {
                d.push(payer(a));
            }
            d.push(format!("Contract: {}", hex::encode(contract_id)));
            d.push(format!("Function: {fn_name}"));
            if *deposit > Dusk::from(0) {
                d.push(format!("Deposit: {deposit} DUSK"));
            }
            d.push(fee(gas_limit, gas_price));
        }
        Command::Export {
            profile_idx, dir, ..
        } => {
            d.push(format!("Profile: {}", profile_idx.unwrap_or(0)));
            d.push(format!("Directory: {}", dir.display()));
        }
        _ => {}
    }

    d
}
