// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::process::Command as ProcessCommand;
use std::time::{Duration, Instant};

use base64::Engine;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dusk_core::stake::StakeData;
use rusk_wallet::currency::Dusk;
use rusk_wallet::{Address, MAX_PROFILES, Profile, Wallet};
use wallet_core::BalanceInfo;

use super::action::AsyncResult;
use super::forms::{self, FormId, FormState};
use crate::command::TransactionHistory;
use crate::settings::Settings;
use crate::{Command, WalletFile};

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

/// Cached stake state together with the last tip height it was attempted for.
#[derive(Debug, Clone, Default)]
pub struct CachedStakeInfo {
    pub state: Option<StakeState>,
    pub attempted_at_tip: Option<u64>,
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

const SYNCING_STATUS_GRACE_AFTER_COMPLETE: Duration = Duration::from_secs(8);
const MAX_DISPLAYABLE_ERROR_LEN: usize = 180;

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
    StakeInfo { owner: Option<String> },
    /// Full address display (replaces menu area)
    Addresses,
    /// Help overlay showing keybindings
    Help,
}

/// Dashboard menu items. Order matches the menu list.
/// The "p" entry label is overridden dynamically by `App::menu_label()`.
pub const MENU_ITEMS: &[(&str, &str)] = &[
    ("t", "Transfer DUSK"),
    ("v", "View Addresses"),
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
    ("m", "Import Different Wallet"),
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
    stake_info: HashMap<u8, CachedStakeInfo>,
    pub sync_status: SyncStatus,
    pub sync_block_height: Option<u64>,
    pub network_label: String,
    pub connection: ConnectionStatus,
    last_sync_complete_at: Option<Instant>,

    // Pending command for confirmation flow
    pub pending_cmd: Option<Command>,
    pub pending_cmd_description: Vec<String>,

    // Status messages from wallet operations
    pub status_messages: Vec<String>,

    // History scroll state
    pub history_selected: usize,

    // Last submitted form (for retry on error)
    pub last_form: Option<Box<FormState>>,

    // Clipboard feedback shown in the hint bar (message + time it was set)
    pub clipboard_msg: Option<(String, Instant)>,

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
            sync_block_height: None,
            network_label: resolve_network_label(settings),
            connection: ConnectionStatus::default(),
            last_sync_complete_at: None,
            pending_cmd: None,
            pending_cmd_description: Vec::new(),
            status_messages: Vec::new(),
            history_selected: 0,
            last_form: None,
            clipboard_msg: None,
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
            AppScreen::History { .. } | AppScreen::StakeInfo { .. } => {
                self.handle_history_key(key);
                None
            }
            AppScreen::Addresses => {
                self.handle_addresses_key(key);
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
            "s" => Some(AppAction::OpenStakeForm),
            "u" => {
                self.open_form(FormId::Unstake);
                None
            }
            "c" => Some(AppAction::OpenClaimRewardsForm),
            "d" => {
                self.open_form(FormId::Shield);
                None
            }
            "e" => {
                self.open_form(FormId::Unshield);
                None
            }
            "h" => Some(AppAction::FetchHistory),
            "v" => {
                self.screen = AppScreen::Addresses;
                None
            }
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
            KeyCode::Tab => {
                form.next_field();
            }
            KeyCode::BackTab => {
                form.prev_field();
            }
            KeyCode::Down => {
                form.next_field();
            }
            KeyCode::Up => {
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
                    let desc = self.pending_cmd_description.join("\n");
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
                ) && let Some(form) = self.last_form.take()
                {
                    self.screen = AppScreen::Form { form };
                    return;
                }
                self.screen = AppScreen::Dashboard;
            }
            KeyCode::Char('o') => {
                let explorer_url = match &self.screen {
                    AppScreen::Result {
                        info:
                            ResultInfo::TxSent {
                                explorer_url: Some(url),
                                ..
                            }
                            | ResultInfo::DeployTxSent {
                                explorer_url: Some(url),
                                ..
                            },
                    } => Some(url.clone()),
                    _ => None,
                };

                if let Some(url) = explorer_url {
                    match open_explorer_url(&url) {
                        Ok(()) => self.screen = AppScreen::Dashboard,
                        Err(err) => {
                            self.screen = AppScreen::Result {
                                info: ResultInfo::Error {
                                    message: format!(
                                        "Could not open explorer automatically.\n\
                                         Copy this URL in your browser:\n{url}\n\
                                         Launcher error:\n{err}"
                                    ),
                                },
                            };
                        }
                    }
                } else {
                    self.screen = AppScreen::Result {
                        info: ResultInfo::Error {
                            message:
                                "No explorer URL configured for this network."
                                    .into(),
                        },
                    };
                }
            }
            _ => {}
        }
    }

    fn handle_addresses_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.clipboard_msg = None;
                self.screen = AppScreen::Dashboard;
            }
            KeyCode::Char('s') => {
                let addr =
                    Address::Shielded(self.current_profile().shielded_addr)
                        .to_string();
                self.clipboard_msg = Some((
                    clipboard_message("Shielded", copy_to_clipboard(&addr)),
                    Instant::now(),
                ));
            }
            KeyCode::Char('p') => {
                let addr = Address::Public(self.current_profile().public_addr)
                    .to_string();
                self.clipboard_msg = Some((
                    clipboard_message("Public", copy_to_clipboard(&addr)),
                    Instant::now(),
                ));
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
                if let AppScreen::History { entries } = &self.screen
                    && self.history_selected + 1 < entries.len()
                {
                    self.history_selected += 1;
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
            AsyncResult::StakeUpdate {
                profile_idx,
                stake,
                attempted_at_tip,
            } => {
                let entry = self.stake_info.entry(profile_idx).or_default();
                apply_stake_update(entry, stake, attempted_at_tip);
            }
            AsyncResult::SyncStatus(msg) | AsyncResult::StatusMessage(msg) => {
                if let Some(height) = parse_block_height(&msg) {
                    self.sync_block_height = Some(
                        self.sync_block_height
                            .map_or(height, |current| current.max(height)),
                    );
                }

                if msg.contains("Complete") || msg.contains("complete") {
                    self.sync_status = SyncStatus::Synced;
                    self.last_sync_complete_at = Some(Instant::now());
                } else if msg.contains("Error") || msg.contains("error") {
                    self.sync_status = SyncStatus::Error(msg.clone());
                    self.last_sync_complete_at = None;
                } else if msg.contains("Syncing") || msg.contains("syncing") {
                    self.transition_to_syncing();
                }
                self.push_status(msg);
            }
            AsyncResult::ChainTipHeight(height) => {
                self.sync_block_height = Some(
                    self.sync_block_height
                        .map_or(height, |current| current.max(height)),
                );
            }
            AsyncResult::TxComplete(hash) => {
                let tx_hash = hex::encode(hash.to_bytes());
                let explorer_url = self
                    .settings
                    .explorer
                    .as_ref()
                    .map(|e| format!("{e}{tx_hash}"));
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
                    .map(|e| format!("{e}{tx_hash}"));
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
                    info: ResultInfo::Error {
                        message: sanitize_error_for_display(&msg),
                    },
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

    fn transition_to_syncing(&mut self) {
        let should_switch = match self.sync_status {
            SyncStatus::Unknown
            | SyncStatus::Error(_)
            | SyncStatus::Syncing => true,
            SyncStatus::Synced => self
                .last_sync_complete_at
                .map(|at| at.elapsed() >= SYNCING_STATUS_GRACE_AFTER_COMPLETE)
                .unwrap_or(true),
        };

        if should_switch {
            self.sync_status = SyncStatus::Syncing;
        }
    }

    /// Return the clipboard feedback message if it hasn't expired yet.
    pub fn clipboard_message(&self) -> Option<&str> {
        const CLIPBOARD_MSG_TTL: Duration = Duration::from_secs(3);
        self.clipboard_msg
            .as_ref()
            .filter(|(_, at)| at.elapsed() < CLIPBOARD_MSG_TTL)
            .map(|(msg, _)| msg.as_str())
    }

    /// Clear the clipboard feedback message if it has expired.
    pub fn expire_clipboard_msg(&mut self) {
        if let Some((_, at)) = &self.clipboard_msg {
            const CLIPBOARD_MSG_TTL: Duration = Duration::from_secs(3);
            if at.elapsed() >= CLIPBOARD_MSG_TTL {
                self.clipboard_msg = None;
            }
        }
    }

    pub fn open_form(&mut self, form_id: FormId) {
        let (phoenix_spendable, moonlight_bal) = self.form_balances();
        let form = forms::build_form(
            form_id,
            self.profile_idx,
            phoenix_spendable,
            moonlight_bal,
            self.claim_rewards_max(),
            self.wallet.profiles(),
            &self.settings.wallet_dir,
        );

        self.screen = AppScreen::Form {
            form: Box::new(form),
        };
    }

    pub fn open_stake_form(&mut self, locked_owner: Option<Address>) {
        let (phoenix_spendable, moonlight_bal) = self.form_balances();
        let mut form = forms::build_form(
            FormId::Stake,
            self.profile_idx,
            phoenix_spendable,
            moonlight_bal,
            self.claim_rewards_max(),
            self.wallet.profiles(),
            &self.settings.wallet_dir,
        );

        if let Some(owner) = locked_owner {
            form.lock_stake_owner(&owner);
        }

        self.screen = AppScreen::Form {
            form: Box::new(form),
        };
    }

    pub fn open_claim_rewards_form(&mut self, reward_max: Option<Dusk>) {
        let (phoenix_spendable, moonlight_bal) = self.form_balances();
        let form = forms::build_form(
            FormId::ClaimRewards,
            self.profile_idx,
            phoenix_spendable,
            moonlight_bal,
            reward_max,
            self.wallet.profiles(),
            &self.settings.wallet_dir,
        );

        self.screen = AppScreen::Form {
            form: Box::new(form),
        };
    }

    fn claim_rewards_max(&self) -> Option<Dusk> {
        self.current_stake_state().map(|stake| match stake {
            StakeState::Loaded(data) => Dusk::from(data.reward),
            StakeState::NoStake => Dusk::from(0),
        })
    }

    fn form_balances(&self) -> (Dusk, Dusk) {
        let bal = self.current_balance();
        let phoenix_spendable = bal
            .phoenix
            .as_ref()
            .map(|b| Dusk::from(b.spendable))
            .unwrap_or(Dusk::from(0));
        let moonlight_bal = bal.moonlight.unwrap_or(Dusk::from(0));

        (phoenix_spendable, moonlight_bal)
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

    pub fn current_stake_state(&self) -> Option<&StakeState> {
        self.stake_info
            .get(&self.profile_idx)
            .and_then(|info| info.state.as_ref())
    }

    pub fn current_profile_stake_refresh_tip_to_fetch(&self) -> Option<u64> {
        stake_refresh_tip_to_fetch(
            self.sync_block_height,
            self.stake_info
                .get(&self.profile_idx)
                .and_then(|info| info.attempted_at_tip),
        )
    }

    pub fn note_current_profile_stake_refresh_attempt(
        &mut self,
        tip_height: u64,
    ) {
        self.stake_info
            .entry(self.profile_idx)
            .or_default()
            .attempted_at_tip = Some(tip_height);
    }

    /// Build confirmation details for a command.
    pub fn prepare_confirmation(&mut self, cmd: Command) {
        // Save the current form for retry on error
        if let AppScreen::Form { form } = &self.screen {
            self.last_form = Some(form.clone());
        }
        if let Some(err) = self.balance_precheck_error(&cmd) {
            self.pending_cmd = None;
            self.pending_cmd_description.clear();
            self.screen = AppScreen::Result {
                info: ResultInfo::Error { message: err },
            };
            return;
        }
        let details = build_confirmation_details(&cmd, self.wallet);
        self.pending_cmd_description = details;
        self.pending_cmd = Some(cmd);
        self.screen = AppScreen::Confirmation;
    }

    fn balance_precheck_error(&self, cmd: &Command) -> Option<String> {
        let current = self.current_balance();
        let moonlight_bal = current.moonlight;
        let phoenix_spendable = current
            .phoenix
            .as_ref()
            .map(|bal| Dusk::from(bal.spendable));

        let (balance_type, to_deduct) = max_deduction(cmd);
        match balance_type {
            BalanceType::Public => {
                let spendable = moonlight_bal?;
                (spendable < to_deduct).then(|| {
                    format!(
                        "Balance is not enough to cover amount + max fee. \
                         You need {} more DUSK.",
                        to_deduct - spendable
                    )
                })
            }
            BalanceType::Shielded => {
                let spendable = phoenix_spendable?;
                (spendable < to_deduct).then(|| {
                    format!(
                        "Balance is not enough to cover amount + max fee. \
                         You need {} more DUSK.",
                        to_deduct - spendable
                    )
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum BalanceType {
    Public,
    Shielded,
}

fn max_deduction(cmd: &Command) -> (BalanceType, Dusk) {
    match cmd {
        Command::Shield { amt, .. }
        | Command::Unshield { amt, .. }
        | Command::ContractCall { deposit: amt, .. }
        | Command::Stake { amt, .. }
        | Command::Transfer { amt, .. } => {
            let (bal_type, fee) = max_fee(cmd);
            (bal_type, fee + *amt)
        }
        Command::Balance { .. }
        | Command::Blob { .. }
        | Command::CalculateContractId { .. }
        | Command::ClaimRewards { .. }
        | Command::Create { .. }
        | Command::Restore { .. }
        | Command::Settings
        | Command::Export { .. }
        | Command::History { .. }
        | Command::Profiles { .. }
        | Command::Withdraw { .. }
        | Command::StakeInfo { .. }
        | Command::Unstake { .. }
        | Command::ContractDeploy { .. }
        | Command::DriverDeploy { .. } => max_fee(cmd),
    }
}

fn max_fee(cmd: &Command) -> (BalanceType, Dusk) {
    let gas_fee = |gas_limit: &u64, gas_price: &u64| {
        Dusk::from(gas_limit.saturating_mul(*gas_price))
    };

    match cmd {
        Command::Blob {
            address,
            gas_limit,
            gas_price,
            ..
        }
        | Command::Withdraw {
            address,
            gas_limit,
            gas_price,
            ..
        }
        | Command::ClaimRewards {
            address,
            gas_limit,
            gas_price,
            ..
        }
        | Command::ContractDeploy {
            address,
            gas_limit,
            gas_price,
            ..
        }
        | Command::ContractCall {
            address,
            gas_limit,
            gas_price,
            ..
        }
        | Command::Stake {
            address,
            gas_limit,
            gas_price,
            ..
        }
        | Command::Transfer {
            sender: address,
            gas_limit,
            gas_price,
            ..
        }
        | Command::Unstake {
            address,
            gas_limit,
            gas_price,
            ..
        } => match address {
            Some(Address::Shielded(_)) => {
                (BalanceType::Shielded, gas_fee(gas_limit, gas_price))
            }
            Some(Address::Public(_)) | None => {
                (BalanceType::Public, gas_fee(gas_limit, gas_price))
            }
        },
        Command::Shield {
            gas_limit,
            gas_price,
            ..
        } => (BalanceType::Public, gas_fee(gas_limit, gas_price)),
        Command::Unshield {
            gas_limit,
            gas_price,
            ..
        } => (BalanceType::Shielded, gas_fee(gas_limit, gas_price)),
        Command::Settings
        | Command::CalculateContractId { .. }
        | Command::Create { .. }
        | Command::Restore { .. }
        | Command::StakeInfo { .. }
        | Command::Profiles { .. }
        | Command::Balance { .. }
        | Command::History { .. }
        | Command::Export { .. }
        | Command::DriverDeploy { .. } => (BalanceType::Public, Dusk::from(0)),
    }
}

fn sanitize_error_for_display(message: &str) -> String {
    let mut first_line =
        message.lines().next().unwrap_or("").trim().to_string();

    if first_line.is_empty() {
        return "Operation failed. See logs for details.".into();
    }

    if first_line.contains('/')
        || first_line.contains('\\')
        || first_line.contains("://")
    {
        return "Operation failed. See logs for details.".into();
    }

    if first_line.len() > MAX_DISPLAYABLE_ERROR_LEN {
        first_line.truncate(MAX_DISPLAYABLE_ERROR_LEN - 3);
        first_line.push_str("...");
    }

    first_line
}

fn clipboard_message(label: &str, result: Result<(), String>) -> String {
    match result {
        Ok(()) => format!("{label} address: copy request sent to terminal"),
        Err(err) => {
            format!(
                "{label} address: failed to send copy request to terminal: {err}"
            )
        }
    }
}

fn apply_stake_update(
    entry: &mut CachedStakeInfo,
    stake: StakeState,
    attempted_at_tip: Option<u64>,
) {
    entry.state = Some(stake);
    if let Some(tip_height) = attempted_at_tip {
        entry.attempted_at_tip = Some(tip_height);
    }
}

/// Send a text to the clipboard using OSC 52 escape sequence. This allows
/// copying without needing external clipboard tools.
///
/// Not every terminal supports OSC 52. And OSC 52 is fire-and-forget.
/// Success cannot be confirmed.
fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;
    let osc52 = format!(
        "\x1b]52;c;{}\x07",
        base64::engine::general_purpose::STANDARD.encode(text.as_bytes())
    );
    // Write to stderr: ratatui owns stdout (raw/alternate-screen mode), but
    // stderr is a separate fd that still reaches the same terminal device.
    std::io::stderr()
        .write_all(osc52.as_bytes())
        .map_err(|e| e.to_string())
}

fn open_explorer_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        open_explorer_url_linux(url)
    }

    #[cfg(not(target_os = "linux"))]
    {
        open::that(url).map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "linux")]
fn open_explorer_url_linux(url: &str) -> Result<(), String> {
    if is_wsl() {
        if try_open_with_command("wslview", &[url]).is_ok() {
            return Ok(());
        }
        if try_open_with_command("cmd.exe", &["/C", "start", "", url]).is_ok() {
            return Ok(());
        }
        if try_open_with_powershell(url).is_ok() {
            return Ok(());
        }
    }

    if let Ok(browser) = std::env::var("BROWSER") {
        for candidate in browser
            .split(':')
            .map(str::trim)
            .filter(|candidate| !candidate.is_empty())
        {
            if try_open_with_browser_candidate(candidate, url).is_ok() {
                return Ok(());
            }
        }
    }

    for (cmd, args) in [
        ("xdg-open", vec![url]),
        ("sensible-browser", vec![url]),
        ("gio", vec!["open", url]),
    ] {
        if try_open_with_command(cmd, &args).is_ok() {
            return Ok(());
        }
    }

    if open::that(url).is_ok() {
        return Ok(());
    }

    Err("No supported browser launcher succeeded on this environment.".into())
}

#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::env::var("WSL_DISTRO_NAME").is_ok()
        || std::env::var("WSL_INTEROP").is_ok()
}

#[cfg(target_os = "linux")]
fn try_open_with_browser_candidate(
    candidate: &str,
    url: &str,
) -> Result<(), String> {
    let mut parts = candidate.split_whitespace();
    let cmd = parts
        .next()
        .ok_or_else(|| "BROWSER candidate is empty".to_string())?;

    let mut args: Vec<String> = parts.map(str::to_owned).collect();
    if args.is_empty() {
        args.push(url.to_string());
    } else {
        let mut replaced = false;
        for arg in &mut args {
            if arg.contains("%s") {
                *arg = arg.replace("%s", url);
                replaced = true;
            }
        }
        if !replaced {
            args.push(url.to_string());
        }
    }

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    try_open_with_command(cmd, &arg_refs)
        .map_err(|err| format!("BROWSER candidate '{candidate}' failed: {err}"))
}

#[cfg(target_os = "linux")]
fn try_open_with_command(cmd: &str, args: &[&str]) -> Result<(), String> {
    let output = ProcessCommand::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("{cmd}: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!("{cmd} exited with {}", output.status))
        } else {
            Err(format!("{cmd}: {stderr}"))
        }
    }
}

#[cfg(target_os = "linux")]
fn try_open_with_powershell(url: &str) -> Result<(), String> {
    let script = format!("Start-Process '{}'", url.replace('\'', "''"));
    let output = ProcessCommand::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| format!("powershell.exe: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("powershell.exe exited with {}", output.status))
    }
}

/// Actions the TUI event loop should take after handling input.
pub enum AppAction {
    RefreshBalance,
    FetchHistory,
    FetchStakeInfo,
    OpenStakeForm,
    OpenClaimRewardsForm,
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

fn parse_block_height(message: &str) -> Option<u64> {
    let mut parts = message.split_whitespace();
    while let Some(part) = parts.next() {
        if part.eq_ignore_ascii_case("block") {
            let raw = parts.next()?;
            let digits = raw.trim_matches(|c: char| !c.is_ascii_digit());
            if !digits.is_empty() {
                let height = digits.parse::<u64>().ok()?;
                return (height > 0).then_some(height);
            }
        }
    }
    None
}

fn stake_refresh_tip_to_fetch(
    current_tip: Option<u64>,
    last_attempted_tip: Option<u64>,
) -> Option<u64> {
    match (current_tip, last_attempted_tip) {
        (Some(current_tip), Some(last_attempted_tip))
            if last_attempted_tip >= current_tip =>
        {
            None
        }
        (Some(current_tip), _) => Some(current_tip),
        (None, _) => None,
    }
}

fn resolve_network_label(settings: &Settings) -> String {
    if let Some(name) = settings.network_name.as_deref() {
        return name.to_string();
    }

    infer_network_label(&settings.state).to_string()
}

fn infer_network_label(state_url: &url::Url) -> &'static str {
    let host = state_url
        .host_str()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if host.contains("testnet") {
        "Testnet"
    } else if host.contains("devnet") {
        "Devnet"
    } else if host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.ends_with(".local")
    {
        "Local"
    } else if host.contains("mainnet") || host == "nodes.dusk.network" {
        "Mainnet"
    } else {
        "Custom"
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use rusk_wallet::Profile;
    use rusk_wallet::currency::Dusk;
    use wallet_core::Seed;
    use wallet_core::keys::{derive_bls_pk, derive_phoenix_pk};

    use super::{
        App, CachedStakeInfo, StakeState, apply_stake_update,
        stake_refresh_tip_to_fetch,
    };
    use crate::tui::forms::{FormId, build_form};

    fn test_profile_at(index: u8) -> Profile {
        let seed: Seed = [7u8; 64];
        Profile {
            shielded_addr: derive_phoenix_pk(&seed, index),
            public_addr: derive_bls_pk(&seed, index),
        }
    }

    #[test]
    fn missing_stake_info_is_refreshable() {
        assert_eq!(stake_refresh_tip_to_fetch(Some(42), None), Some(42));
    }

    #[test]
    fn current_tip_is_not_retried_after_already_refreshing_it() {
        assert_eq!(stake_refresh_tip_to_fetch(Some(42), Some(42)), None);
    }

    #[test]
    fn newer_tip_marks_stake_info_as_stale() {
        assert_eq!(stake_refresh_tip_to_fetch(Some(42), Some(41)), Some(42));
    }

    #[test]
    fn stake_update_uses_fetch_attempt_tip() {
        let mut entry = CachedStakeInfo {
            state: None,
            attempted_at_tip: Some(44),
        };

        apply_stake_update(&mut entry, StakeState::NoStake, Some(42));

        assert!(matches!(entry.state, Some(StakeState::NoStake)));
        assert_eq!(entry.attempted_at_tip, Some(42));
    }

    #[test]
    fn stake_update_preserves_attempt_when_result_has_no_tip() {
        let mut entry = CachedStakeInfo {
            state: None,
            attempted_at_tip: Some(42),
        };

        apply_stake_update(&mut entry, StakeState::NoStake, None);

        assert!(matches!(entry.state, Some(StakeState::NoStake)));
        assert_eq!(entry.attempted_at_tip, Some(42));
    }

    #[test]
    fn right_arrow_cycles_focused_stake_owner_field() {
        let temp_dir = std::env::temp_dir();
        let profiles = vec![test_profile_at(0), test_profile_at(1)];
        let mut form = build_form(
            FormId::Stake,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &profiles,
            temp_dir.as_path(),
        );
        let owner_idx = form
            .fields
            .iter()
            .position(|field| field.name == "owner")
            .expect("stake form should have owner field");
        form.focused = owner_idx;

        App::handle_form_key(
            &mut form,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &None,
        );

        assert_eq!(form.fields[owner_idx].selected_option, Some(1));
    }

    #[test]
    fn down_arrow_moves_past_focused_stake_owner_field() {
        let temp_dir = std::env::temp_dir();
        let profiles = vec![test_profile_at(0), test_profile_at(1)];
        let mut form = build_form(
            FormId::Stake,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &profiles,
            temp_dir.as_path(),
        );
        let owner_idx = form
            .fields
            .iter()
            .position(|field| field.name == "owner")
            .expect("stake form should have owner field");
        form.focused = owner_idx;

        App::handle_form_key(
            &mut form,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            &None,
        );

        assert_eq!(form.focused, owner_idx + 1);
        assert_eq!(form.fields[owner_idx].selected_option, Some(0));
    }

    #[test]
    fn up_arrow_moves_before_focused_stake_owner_field() {
        let temp_dir = std::env::temp_dir();
        let profiles = vec![test_profile_at(0), test_profile_at(1)];
        let mut form = build_form(
            FormId::Stake,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &profiles,
            temp_dir.as_path(),
        );
        let owner_idx = form
            .fields
            .iter()
            .position(|field| field.name == "owner")
            .expect("stake form should have owner field");
        form.focused = owner_idx;

        App::handle_form_key(
            &mut form,
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            &None,
        );

        assert_eq!(form.focused, owner_idx - 1);
        assert_eq!(form.fields[owner_idx].selected_option, Some(0));
    }

    #[test]
    fn arrows_on_single_stake_owner_show_no_additional_profile_error() {
        let temp_dir = std::env::temp_dir();
        let profiles = vec![test_profile_at(0)];
        let mut form = build_form(
            FormId::Stake,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &profiles,
            temp_dir.as_path(),
        );
        let owner_idx = form
            .fields
            .iter()
            .position(|field| field.name == "owner")
            .expect("stake form should have owner field");
        form.focused = owner_idx;

        for key in [KeyCode::Left, KeyCode::Right] {
            form.error = None;

            App::handle_form_key(
                &mut form,
                KeyEvent::new(key, KeyModifiers::NONE),
                &None,
            );

            assert_eq!(
                form.error.as_deref(),
                Some("No additional profile created")
            );
            assert_eq!(form.fields[owner_idx].selected_option, Some(0));
        }
    }

    #[test]
    fn number_key_does_not_select_focused_stake_owner_profile() {
        let temp_dir = std::env::temp_dir();
        let profiles = vec![test_profile_at(0), test_profile_at(1)];
        let mut form = build_form(
            FormId::Stake,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &profiles,
            temp_dir.as_path(),
        );
        let owner_idx = form
            .fields
            .iter()
            .position(|field| field.name == "owner")
            .expect("stake form should have owner field");
        form.focused = owner_idx;

        App::handle_form_key(
            &mut form,
            KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE),
            &None,
        );

        assert_eq!(form.fields[owner_idx].selected_option, Some(0));
    }
}
