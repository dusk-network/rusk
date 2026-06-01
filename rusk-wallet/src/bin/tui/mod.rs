// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod action;
mod app;
mod event;
pub mod forms;
mod render;
mod theme;

use std::io::{self, stdout};
use std::time::Duration;

use bip39::{Language, Mnemonic, MnemonicType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{STAKE_CONTRACT, StakeData};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use ratatui::widgets::Clear;
use rkyv::Deserialize;
use rocksdb::ErrorKind;
use rusk_wallet::dat::{self, LATEST_VERSION};
use rusk_wallet::{
    Address, Error as WalletError, GraphQL, Profile, RuesHttpClient, Wallet,
    WalletPath,
};
use tokio::sync::mpsc;
use tracing::debug;
use zeroize::{Zeroize, Zeroizing};

pub use self::action::tui_status;
use self::action::{AsyncResult, clear_status_channel, init_status_channel};
use self::app::{App, AppAction, AppScreen, ConnectionStatus};
use crate::WalletFile;
use crate::command::{gen_iv, gen_salt};
use crate::frontend::OperationResult;
use crate::io::prompt;
use crate::settings::Settings;

const TIP_HEIGHT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const TIP_HEIGHT_POLL_TIMEOUT: Duration = Duration::from_secs(4);
const STAKE_INFO_FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PASSWORD_ATTEMPTS: u8 = 3;
const PRE_WALLET_TICK_RATE: Duration = Duration::from_millis(100);

/// Signals from `run_inner` about why it exited.
enum ExitReason {
    Quit,
    ImportWallet,
}

/// Control flow from a screen-specific key handler.
enum ScreenFlow<R> {
    Continue,
    Done(R),
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WalletSetupChoice {
    Create,
    Restore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WalletSetupAction {
    Continue,
    Cancel,
    Select(WalletSetupChoice),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GeneratedMnemonicVerification {
    Match,
    Invalid,
    Mismatch,
}

#[derive(Clone, Copy)]
struct MnemonicEntryScreenConfig {
    right_title: &'static str,
    heading: &'static str,
    helper: Option<&'static str>,
    enter_hint: &'static str,
    escape_hint: &'static str,
    mask_input: bool,
    content_height: u16,
    width_pct: u16,
}

struct MnemonicEntryState {
    input: Zeroizing<String>,
    cursor: usize,
    error: Option<String>,
}

const RESTORE_MNEMONIC_SCREEN: MnemonicEntryScreenConfig =
    MnemonicEntryScreenConfig {
        right_title: "Restore",
        heading: "Enter your 12-word mnemonic phrase:",
        helper: None,
        enter_hint: "Submit",
        escape_hint: "Back",
        mask_input: true,
        content_height: 10,
        width_pct: 80,
    };

const VERIFY_GENERATED_MNEMONIC_SCREEN: MnemonicEntryScreenConfig =
    MnemonicEntryScreenConfig {
        right_title: "Verify Backup",
        heading: "Re-enter the generated mnemonic phrase exactly as shown:",
        helper: Some(
            "Use the same single-string format with spaces between words.",
        ),
        enter_hint: "Continue",
        escape_hint: "Back",
        mask_input: false,
        content_height: 11,
        width_pct: 84,
    };

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SyncStage {
    #[default]
    Starting,
    CachedPosition,
    FetchingFreshNotes,
    StreamingNotes,
    Complete,
    Error,
}

impl SyncStage {
    const TOTAL_STEPS: usize = 5;

    fn index(self) -> usize {
        match self {
            Self::Starting => 1,
            Self::CachedPosition => 2,
            Self::FetchingFreshNotes => 3,
            Self::StreamingNotes => 4,
            Self::Complete => 5,
            Self::Error => 5,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Starting => "Initializing sync session",
            Self::CachedPosition => "Reading cached note position",
            Self::FetchingFreshNotes => "Requesting note stream from node",
            Self::StreamingNotes => "Streaming and decoding notes",
            Self::Complete => "Initial sync complete",
            Self::Error => "Sync error",
        }
    }

    fn from_status(message: &str) -> Option<Self> {
        let lower = message.to_ascii_lowercase();
        if lower.contains("error") {
            return Some(Self::Error);
        }
        if lower.contains("syncing complete")
            || lower.contains("initial chain sync complete")
        {
            return Some(Self::Complete);
        }
        if lower.contains("getting cached note position") {
            return Some(Self::CachedPosition);
        }
        if lower.contains("fetching fresh notes") {
            return Some(Self::FetchingFreshNotes);
        }
        if lower.contains("streaming notes")
            || lower.contains("syncing chain state at block")
        {
            return Some(Self::StreamingNotes);
        }
        if lower.contains("connection established")
            || lower.contains("resuming sync from cached note position")
        {
            return Some(Self::FetchingFreshNotes);
        }
        None
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Error)
    }

    fn tracks_block_progress(self) -> bool {
        matches!(self, Self::StreamingNotes | Self::Complete)
    }
}

#[derive(Default)]
struct StartupSyncProgress {
    latest_status: String,
    recent_messages: Vec<String>,
    block_height: Option<u64>,
    stream_start_block: Option<u64>,
    stage: SyncStage,
    spinner_frame: usize,
}

impl StartupSyncProgress {
    fn push_status(&mut self, message: String) {
        if let Some(next_stage) = SyncStage::from_status(&message) {
            self.advance_stage(next_stage);
        }

        if let Some(height) = parse_block_height(&message) {
            self.block_height = Some(height);
            if self.stage.tracks_block_progress() {
                self.stream_start_block.get_or_insert(height);
            }
        }

        self.latest_status = message.clone();
        self.recent_messages.push(message);
        if self.recent_messages.len() > 8 {
            self.recent_messages.remove(0);
        }
    }

    fn advance_stage(&mut self, next_stage: SyncStage) {
        if self.stage.is_terminal() {
            return;
        }

        match next_stage {
            SyncStage::Complete | SyncStage::Error => self.stage = next_stage,
            _ if next_stage.index() > self.stage.index() => {
                self.stage = next_stage;
            }
            _ => {}
        }
    }

    fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 4;
    }

    fn progress_percent(&self) -> usize {
        if SyncStage::TOTAL_STEPS <= 1 {
            return 100;
        }

        let capped = self.stage.index().min(SyncStage::TOTAL_STEPS);
        ((capped.saturating_sub(1)) * 100) / (SyncStage::TOTAL_STEPS - 1)
    }

    fn streamed_blocks(&self) -> Option<u64> {
        let start = self.stream_start_block?;
        let current = self.block_height?;
        Some(current.saturating_sub(start))
    }
}

/// Run the TUI interactive mode.
///
/// Handles the full lifecycle: password entry, wallet loading, connection,
/// and the main dashboard event loop.
pub async fn run(
    wallet_path: &WalletPath,
    settings: &Settings,
) -> anyhow::Result<()> {
    // Set up terminal
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    let result = loop {
        match run_inner(&mut terminal, wallet_path, settings).await {
            Ok(ExitReason::Quit) => break Ok(()),
            Ok(ExitReason::ImportWallet) => {
                // Replacement wallet is already saved; restart into it.
                continue;
            }
            Err(e) => break Err(e),
        }
    };

    // Restore terminal
    let _ = terminal.draw(|frame| {
        frame.render_widget(Clear, frame.area());
    });
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    clear_status_channel();

    result
}

/// Back up existing wallet.dat to wallet.dat.old.
fn backup_wallet(wallet_path: &WalletPath) -> anyhow::Result<()> {
    let src = wallet_path.inner();
    if src.exists() {
        let mut dst = src.to_path_buf();
        dst.set_extension("dat.old");
        std::fs::copy(src, &dst).map_err(|e| {
            anyhow::anyhow!(
                "Failed to back up wallet file {} -> {}: {}",
                src.display(),
                dst.display(),
                e
            )
        })?;
    }
    Ok(())
}

/// Remove the wallet cache directory if present.
fn clear_wallet_cache(wallet_path: &WalletPath) -> anyhow::Result<()> {
    let cache_dir = wallet_path.cache_dir();
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).map_err(|e| {
            anyhow::anyhow!(
                "Failed to clear cache directory {}: {}",
                cache_dir.display(),
                e
            )
        })?;
    }
    Ok(())
}

fn should_reset_cache_on_connect_error(err: &WalletError) -> bool {
    match err {
        WalletError::RocksDB(db_err) => {
            matches!(
                db_err.kind(),
                ErrorKind::InvalidArgument | ErrorKind::Corruption
            )
        }
        _ => false,
    }
}

fn wallet_setup_action_for_key(key: KeyCode) -> WalletSetupAction {
    match key {
        KeyCode::Enter => WalletSetupAction::Select(WalletSetupChoice::Create),
        KeyCode::Esc => WalletSetupAction::Cancel,
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'c' => WalletSetupAction::Select(WalletSetupChoice::Create),
            'r' => WalletSetupAction::Select(WalletSetupChoice::Restore),
            'q' => WalletSetupAction::Cancel,
            _ => WalletSetupAction::Continue,
        },
        _ => WalletSetupAction::Continue,
    }
}

fn verify_generated_mnemonic_input(
    input: &str,
    expected_phrase: &str,
) -> GeneratedMnemonicVerification {
    match Mnemonic::from_phrase(input.trim(), Language::English) {
        Ok(mnemonic) if mnemonic.phrase() == expected_phrase => {
            GeneratedMnemonicVerification::Match
        }
        Ok(_) => GeneratedMnemonicVerification::Mismatch,
        Err(_) => GeneratedMnemonicVerification::Invalid,
    }
}

fn handle_mnemonic_char_input(state: &mut MnemonicEntryState, c: char) {
    if !c.is_ascii() {
        state.error =
            Some("Mnemonic phrases only accept ASCII characters.".into());
        return;
    }

    state.input.insert(state.cursor, c);
    state.cursor += c.len_utf8();
    state.error = None;
}

fn run_mnemonic_entry<R, Validate>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    screen: MnemonicEntryScreenConfig,
    mut validate: Validate,
) -> anyhow::Result<Option<R>>
where
    Validate: FnMut(&mut MnemonicEntryState) -> anyhow::Result<ScreenFlow<R>>,
{
    run_screen(
        terminal,
        MnemonicEntryState {
            input: Zeroizing::new(String::new()),
            cursor: 0,
            error: None,
        },
        PRE_WALLET_TICK_RATE,
        |frame, s| {
            render::render_mnemonic_entry_screen(
                frame,
                &render::MnemonicEntryView {
                    screen: &screen,
                    input: &s.input,
                    cursor: s.cursor,
                    error: s.error.as_deref(),
                },
            );
        },
        |s, key| match key.code {
            KeyCode::Enter => validate(s),
            KeyCode::Esc => {
                s.input.zeroize();
                Ok(ScreenFlow::Cancel)
            }
            KeyCode::Char(c) => {
                handle_mnemonic_char_input(s, c);
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Backspace => {
                if s.cursor > 0 {
                    s.cursor -= 1;
                    s.input.remove(s.cursor);
                    s.error = None;
                }
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Left => {
                s.cursor = s.cursor.saturating_sub(1);
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Right => {
                if s.cursor < s.input.len() {
                    s.cursor += 1;
                }
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Home => {
                s.cursor = 0;
                Ok(ScreenFlow::Continue)
            }
            KeyCode::End => {
                s.cursor = s.input.len();
                Ok(ScreenFlow::Continue)
            }
            _ => Ok(ScreenFlow::Continue),
        },
    )
}

fn validate_restore_mnemonic_entry(
    state: &mut MnemonicEntryState,
) -> anyhow::Result<ScreenFlow<Zeroizing<String>>> {
    match Mnemonic::from_phrase(state.input.trim(), Language::English) {
        Ok(mnemonic) => {
            let validated = Zeroizing::new(mnemonic.phrase().to_string());
            state.input.zeroize();
            Ok(ScreenFlow::Done(validated))
        }
        Err(_) => {
            state.error = Some(
                "Invalid mnemonic. Enter 12 valid BIP39 words separated by \
                 spaces."
                    .into(),
            );
            Ok(ScreenFlow::Continue)
        }
    }
}

fn validate_generated_mnemonic_entry(
    state: &mut MnemonicEntryState,
    expected_phrase: &str,
) -> anyhow::Result<ScreenFlow<()>> {
    match verify_generated_mnemonic_input(state.input.as_str(), expected_phrase)
    {
        GeneratedMnemonicVerification::Match => {
            state.input.zeroize();
            Ok(ScreenFlow::Done(()))
        }
        GeneratedMnemonicVerification::Invalid => {
            state.error = Some(
                "Invalid mnemonic. Enter the generated 12-word phrase \
                 exactly as shown."
                    .into(),
            );
            Ok(ScreenFlow::Continue)
        }
        GeneratedMnemonicVerification::Mismatch => {
            state.error = Some(
                "Mnemonic does not match the generated phrase. Please try \
                 again."
                    .into(),
            );
            Ok(ScreenFlow::Continue)
        }
    }
}

/// Inner run function. Separated so terminal cleanup always happens.
async fn run_inner(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    wallet_path: &WalletPath,
    settings: &Settings,
) -> anyhow::Result<ExitReason> {
    // If no wallet file exists, offer to create or restore a wallet
    let mut wallet = if !wallet_path.inner().exists() {
        match wallet_setup_flow(terminal, wallet_path, false)? {
            Some(w) => w,
            None => return Ok(ExitReason::Quit), // User cancelled
        }
    } else {
        // Read wallet file metadata (doesn't need password)
        let (file_version, salt_and_iv) =
            dat::read_file_version_and_salt_iv(wallet_path)?;

        // Get password and load wallet (with retry on wrong password)
        let mut pwd_error: Option<&str> = None;
        let mut password_attempts = 0u8;
        let (wallet, password) = loop {
            let mut pwd = match &settings.password {
                Some(p) => p.clone(),
                None => match enter_password(terminal, pwd_error)? {
                    Some(p) => p,
                    None => return Ok(ExitReason::Quit), /* User pressed Esc
                                                          * — quit */
                },
            };

            let key = prompt::derive_key(
                file_version,
                &pwd,
                salt_and_iv.map(|si| si.0).as_ref(),
            )?;

            match Wallet::from_file(WalletFile {
                path: wallet_path.clone(),
                aes_key: key,
                salt: salt_and_iv.map(|si| si.0),
                iv: salt_and_iv.map(|si| si.1),
            }) {
                Ok(w) => break (w, pwd),
                Err(_) if settings.password.is_none() => {
                    pwd.zeroize();
                    password_attempts = password_attempts.saturating_add(1);
                    if password_attempts >= MAX_PASSWORD_ATTEMPTS {
                        return Err(
                            rusk_wallet::Error::AttemptsExhausted.into()
                        );
                    }
                    pwd_error = Some("Wrong password. Please try again.");
                    continue;
                }
                Err(e) => {
                    pwd.zeroize();
                    return Err(e.into());
                }
            }
        };

        // Handle old wallet file version (migration)
        let mut wallet = wallet;
        let wallet_fv = wallet.get_file_version()?;
        if wallet_fv.is_old() {
            crate::update_wallet_file(
                &mut wallet,
                Some(password.as_str()),
                wallet_fv,
            )?;
        }

        // Zeroize the password — no longer needed
        let mut pwd = password;
        pwd.zeroize();

        wallet
    };

    // Phase 3: Connect
    let (tx, mut rx) = mpsc::unbounded_channel::<AsyncResult>();
    init_status_channel(tx.clone());

    if let Err(err) = wallet
        .connect_with_status(
            settings.state.as_str(),
            settings.prover.as_str(),
            settings.archiver.as_str(),
            tui_status,
        )
        .await
    {
        if should_reset_cache_on_connect_error(&err) {
            debug!(
                "[OFFLINE MODE]: connect failed with cache mismatch; \
                 clearing cache and retrying: {err}"
            );

            if let Err(cache_err) = wallet.delete_cache() {
                debug!(
                    "Failed to clear wallet cache for reconnect: {cache_err}"
                );
            } else if let Err(retry_err) = wallet
                .connect_with_status(
                    settings.state.as_str(),
                    settings.prover.as_str(),
                    settings.archiver.as_str(),
                    tui_status,
                )
                .await
            {
                debug!(
                    "[OFFLINE MODE]: reconnect after cache reset failed: \
                     {retry_err}"
                );
            }
        } else {
            debug!("[OFFLINE MODE]: Unable to connect: {err}");
        }
    }

    let connected: bool =
        tokio::time::timeout(Duration::from_secs(5), wallet.is_online())
            .await
            .unwrap_or_default();

    if connected && !run_startup_sync_gate(terminal, &wallet, &mut rx).await? {
        wallet.close();
        return Ok(ExitReason::Quit);
    }

    let _ = wallet.register_sync();

    // Phase 4: Main event loop
    let mut app = App::new(&mut wallet, settings);
    let mut tip_poller_started = false;

    let tick_rate = Duration::from_millis(100);
    let mut needs_initial_fetch = true;

    loop {
        // Render first — never block before drawing
        terminal.draw(|frame| render::render(frame, &app))?;

        // One-time async init after the first frame is visible
        if needs_initial_fetch {
            needs_initial_fetch = false;

            let connected: bool = tokio::time::timeout(
                Duration::from_secs(5),
                app.wallet.is_online(),
            )
            .await
            .unwrap_or_default();
            app.connection = ConnectionStatus { state: connected };

            if connected {
                refresh_dashboard_data(
                    &mut app,
                    "Failed to refresh initial stake info",
                )
                .await;
                if !tip_poller_started {
                    spawn_tip_height_poller(tx.clone(), settings);
                    tip_poller_started = true;
                }
            } else {
                app.handle_async_result(AsyncResult::StatusMessage(
                    "Offline mode: unable to reach node services".into(),
                ));
            }
        }

        // Poll for events
        let action = if let Some(key) = event::poll_event(tick_rate)? {
            app.handle_key(key)
        } else {
            None
        };

        // Drain async results
        while let Ok(result) = rx.try_recv() {
            app.handle_async_result(result);
        }

        // Poll background sync channel
        poll_sync_channel(&mut app);

        if let Some(stake_refresh_tip) =
            app.current_profile_stake_refresh_tip_to_fetch()
        {
            let profile_idx = app.profile_idx;
            let stake_pk = app.current_profile().public_addr;
            app.note_current_profile_stake_refresh_attempt(stake_refresh_tip);
            spawn_stake_info_refresh(
                tx.clone(),
                settings.state.to_string(),
                profile_idx,
                stake_pk,
                Some(stake_refresh_tip),
            );
        }

        // Clear expired transient messages
        app.expire_clipboard_msg();

        // Handle actions
        if let Some(action) = action {
            match action {
                AppAction::RefreshBalance => {
                    refresh_dashboard_data(
                        &mut app,
                        "Failed to refresh stake info after dashboard refresh",
                    )
                    .await;
                }
                AppAction::FetchHistory => {
                    execute_history(&mut app).await;
                }
                AppAction::FetchStakeInfo => {
                    match fetch_stake_info(&mut app).await {
                        Ok(()) => {
                            let owner = fetch_current_stake_owner(&app)
                                .await
                                .map(|owner| {
                                    stake_owner_label(
                                        app.wallet.profiles(),
                                        &owner,
                                    )
                                });
                            app.screen = AppScreen::StakeInfo { owner };
                        }
                        Err(e) => {
                            app.handle_async_result(AsyncResult::Operation(
                                OperationResult::error(e.to_string()),
                            ))
                        }
                    }
                }
                AppAction::OpenStakeForm => {
                    let locked_owner = fetch_current_stake_owner(&app).await;
                    app.open_stake_form(locked_owner);
                }
                AppAction::OpenClaimRewardsForm => {
                    // Fetch fresh stake info so the reward max is up to date,
                    // then open the form
                    match fetch_stake_info(&mut app).await {
                        Ok(()) => app.open_form(forms::FormId::ClaimRewards),
                        Err(err) => {
                            debug!(
                                "Failed to refresh stake info before opening claim rewards form: {err}"
                            );
                            app.open_claim_rewards_form(None);
                        }
                    }
                }
                AppAction::ImportWallet => {
                    match wallet_setup_flow(terminal, wallet_path, true)? {
                        Some(_) => {
                            app.wallet.close();
                            clear_wallet_cache(wallet_path)?;
                            return Ok(ExitReason::ImportWallet);
                        }
                        None => {
                            app.screen = AppScreen::Dashboard;
                        }
                    }
                }
                AppAction::CloseForm => {
                    app.screen = AppScreen::Dashboard;
                }
                AppAction::ConfirmCommand(cmd) => {
                    app.prepare_confirmation(cmd);
                }
                AppAction::ExecuteCommand(cmd) => {
                    // Render once with the executing screen visible before we
                    // block on the command.
                    terminal.draw(|frame| render::render(frame, &app))?;
                    execute_command(&mut app, cmd, &tx).await;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    app.wallet.close();
    Ok(ExitReason::Quit)
}

async fn run_startup_sync_gate(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    wallet: &Wallet<WalletFile>,
    rx: &mut mpsc::UnboundedReceiver<AsyncResult>,
) -> anyhow::Result<bool> {
    let mut progress = StartupSyncProgress::default();
    progress.push_status("Starting initial chain sync...".into());

    let sync_future = wallet.sync();
    tokio::pin!(sync_future);
    let tick_rate = Duration::from_millis(120);

    loop {
        while let Ok(result) = rx.try_recv() {
            ingest_startup_result(&mut progress, result);
        }

        terminal.draw(|frame| {
            let view = render::StartupSyncView {
                latest_status: &progress.latest_status,
                block_height: progress.block_height,
                streamed_blocks: progress.streamed_blocks(),
                recent_messages: &progress.recent_messages,
                stage_label: progress.stage.label(),
                stage_progress: (
                    progress.stage.index(),
                    SyncStage::TOTAL_STEPS,
                ),
                progress_percent: progress.progress_percent(),
                spinner_frame: progress.spinner_frame,
            };
            render::render_startup_sync_screen(frame, &view)
        })?;

        if let Some(key) = event::poll_event(Duration::from_millis(1))? {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('c')
            {
                return Ok(false);
            }

            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                return Ok(false);
            }
        }

        tokio::select! {
            result = &mut sync_future => {
                match result {
                    Ok(()) => {
                        progress.push_status("Initial chain sync complete".into());
                        progress.advance_stage(SyncStage::Complete);
                        return Ok(true);
                    }
                    Err(err) => {
                        progress.push_status(format!(
                            "Initial sync failed, continuing in offline mode: {err}"
                        ));
                        progress.advance_stage(SyncStage::Error);
                        debug!("[OFFLINE MODE]: Initial sync failed: {err}");
                        return Ok(true);
                    }
                }
            }
            _ = tokio::time::sleep(tick_rate) => {
                progress.tick();
            }
        }
    }
}

fn ingest_startup_result(
    progress: &mut StartupSyncProgress,
    result: AsyncResult,
) {
    match result {
        AsyncResult::SyncStatus(msg) | AsyncResult::StatusMessage(msg) => {
            progress.push_status(msg);
        }
        AsyncResult::Operation(OperationResult::Error { message }) => {
            progress.push_status(format!("Error: {message}"));
        }
        AsyncResult::BalanceUpdate { .. }
        | AsyncResult::StakeUpdate { .. }
        | AsyncResult::ChainTipHeight(_)
        | AsyncResult::Operation(_)
        | AsyncResult::HistoryFetched(_) => {}
    }
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

fn spawn_tip_height_poller(
    tx: mpsc::UnboundedSender<AsyncResult>,
    settings: &Settings,
) {
    let state_url = settings.state.to_string();
    let archiver_url = settings.archiver.to_string();

    tokio::spawn(async move {
        let gql = match GraphQL::new(state_url, archiver_url, tui_status) {
            Ok(gql) => gql,
            Err(err) => {
                debug!(
                    "Failed to initialize GraphQL client for tip polling: {err}"
                );
                return;
            }
        };

        loop {
            match tokio::time::timeout(
                TIP_HEIGHT_POLL_TIMEOUT,
                gql.tip_height(),
            )
            .await
            {
                Ok(Ok(height)) => {
                    if tx.send(AsyncResult::ChainTipHeight(height)).is_err() {
                        break;
                    }
                }
                Ok(Err(err)) => {
                    debug!("Failed to fetch chain tip height: {err}");
                }
                Err(_) => {
                    debug!("Timed out while fetching chain tip height");
                }
            }

            tokio::time::sleep(TIP_HEIGHT_POLL_INTERVAL).await;
        }
    });
}

fn spawn_stake_info_refresh(
    tx: mpsc::UnboundedSender<AsyncResult>,
    state_url: String,
    profile_idx: u8,
    stake_pk: BlsPublicKey,
    attempted_at_tip: Option<u64>,
) {
    tokio::spawn(async move {
        let client = match RuesHttpClient::new(state_url) {
            Ok(client) => client,
            Err(err) => {
                debug!(
                    "Failed to initialize HTTP client for automatic stake refresh of profile {profile_idx}: {err}"
                );
                return;
            }
        };

        let stake = match tokio::time::timeout(
            STAKE_INFO_FETCH_TIMEOUT,
            fetch_stake_with_client(&client, &stake_pk),
        )
        .await
        {
            Ok(Ok(stake)) => stake,
            Ok(Err(err)) => {
                debug!(
                    "Failed to auto-refresh stake info for profile {profile_idx}: {err}"
                );
                return;
            }
            Err(_) => {
                debug!(
                    "Timed out while auto-refreshing stake info for profile {profile_idx}"
                );
                return;
            }
        };

        let stake = match stake {
            Some(data) => app::StakeState::Loaded(data),
            None => app::StakeState::NoStake,
        };

        let _ = tx.send(AsyncResult::StakeUpdate {
            profile_idx,
            stake,
            attempted_at_tip,
        });
    });
}

/// Common key-driven screen loop: draw, poll for key presses, handle Ctrl+C.
fn run_screen<S, R, Render, Handle>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut state: S,
    tick_rate: Duration,
    mut render: Render,
    mut handle_key: Handle,
) -> anyhow::Result<Option<R>>
where
    Render: for<'a> FnMut(&mut ratatui::Frame<'a>, &S),
    Handle: FnMut(&mut S, KeyEvent) -> anyhow::Result<ScreenFlow<R>>,
{
    loop {
        terminal.draw(|frame| render(frame, &state))?;

        if let Some(key) = event::poll_event(tick_rate)? {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('c')
            {
                return Ok(None);
            }

            match handle_key(&mut state, key)? {
                ScreenFlow::Continue => {}
                ScreenFlow::Cancel => return Ok(None),
                ScreenFlow::Done(val) => return Ok(Some(val)),
            }
        }
    }
}

/// Show a TUI password entry screen. Returns the password string,
/// or None if the user pressed Esc to cancel.
/// If `error` is Some, it shows an error message (for retry after wrong pw).
fn enter_password(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    initial_error: Option<&str>,
) -> anyhow::Result<Option<String>> {
    struct State {
        password: Zeroizing<String>,
        error: Option<String>,
    }

    run_screen(
        terminal,
        State {
            password: Zeroizing::new(String::new()),
            error: initial_error.map(String::from),
        },
        PRE_WALLET_TICK_RATE,
        |frame, s| {
            render::render_password_screen(
                frame,
                s.password.len(),
                s.error.as_deref(),
            );
        },
        |s, key| match key.code {
            KeyCode::Enter => {
                Ok(ScreenFlow::Done(std::mem::take(&mut *s.password)))
            }
            KeyCode::Esc => {
                s.password.zeroize();
                Ok(ScreenFlow::Cancel)
            }
            KeyCode::Char(c) => {
                s.password.push(c);
                s.error = None;
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Backspace => {
                s.password.pop();
                s.error = None;
                Ok(ScreenFlow::Continue)
            }
            _ => Ok(ScreenFlow::Continue),
        },
    )
}

/// Full create/restore flow for creating/replacing wallet data.
/// Returns the created wallet, or None if the user cancelled.
fn wallet_setup_flow(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    wallet_path: &WalletPath,
    replacing_existing_wallet: bool,
) -> anyhow::Result<Option<Wallet<WalletFile>>> {
    // Step 1: Show setup screen, wait for user to proceed or quit
    let setup_choice =
        match enter_wallet_setup_choice(terminal, replacing_existing_wallet)? {
            Some(choice) => choice,
            None => return Ok(None),
        };

    // Step 2: Create or enter mnemonic phrase
    let mut phrase = match setup_choice {
        WalletSetupChoice::Create => {
            match enter_generated_mnemonic(terminal)? {
                Some(phrase) => phrase,
                None => return Ok(None),
            }
        }
        WalletSetupChoice::Restore => match enter_mnemonic(terminal)? {
            Some(p) => p,
            None => return Ok(None),
        },
    };

    // Step 3: Get new password (with confirmation)
    let mut password = match enter_new_password(terminal)? {
        Some(password) => Zeroizing::new(password),
        None => {
            phrase.zeroize();
            return Ok(None);
        }
    };

    // Step 4: Create the wallet
    let wallet = create_wallet_from_phrase(
        wallet_path,
        replacing_existing_wallet,
        phrase.as_str(),
        password.as_str(),
    );
    phrase.zeroize();
    password.zeroize();

    Ok(Some(wallet?))
}

fn enter_generated_mnemonic(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<Option<Zeroizing<String>>> {
    let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
    let mut phrase = Zeroizing::new(mnemonic.phrase().to_string());

    loop {
        if !confirm_generated_mnemonic_backup(terminal, &phrase)? {
            phrase.zeroize();
            return Ok(None);
        }

        if verify_generated_mnemonic(terminal, &phrase)? {
            return Ok(Some(phrase));
        }
    }
}

fn create_wallet_from_phrase(
    wallet_path: &WalletPath,
    replacing_existing_wallet: bool,
    phrase: &str,
    password: &str,
) -> anyhow::Result<Wallet<WalletFile>> {
    let salt = gen_salt();
    let iv = gen_iv();
    let file_version = dat::FileVersion::RuskBinaryFileFormat(LATEST_VERSION);
    let key = prompt::derive_key(file_version, password, Some(&salt))?;

    let mut wallet = Wallet::new(phrase)?;
    if replacing_existing_wallet {
        backup_wallet(wallet_path)?;
    }
    wallet
        .save_to(WalletFile {
            path: wallet_path.clone(),
            aes_key: key,
            salt: Some(salt),
            iv: Some(iv),
        })
        .inspect_err(|_| wallet.close())?;

    Ok(wallet)
}

/// Show the create/restore setup screen.
/// Returns the selected setup flow, or None if the user cancelled.
fn enter_wallet_setup_choice(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    replacing_existing_wallet: bool,
) -> anyhow::Result<Option<WalletSetupChoice>> {
    run_screen(
        terminal,
        (),
        PRE_WALLET_TICK_RATE,
        |frame, _| {
            render::render_wallet_setup_screen(frame, replacing_existing_wallet)
        },
        |_, key| match wallet_setup_action_for_key(key.code) {
            WalletSetupAction::Continue => Ok(ScreenFlow::Continue),
            WalletSetupAction::Cancel => Ok(ScreenFlow::Cancel),
            WalletSetupAction::Select(choice) => Ok(ScreenFlow::Done(choice)),
        },
    )
}

fn confirm_generated_mnemonic_backup(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    phrase: &str,
) -> anyhow::Result<bool> {
    let res = run_screen(
        terminal,
        (),
        PRE_WALLET_TICK_RATE,
        |frame, _| render::render_generated_mnemonic_screen(frame, phrase),
        |_, key| match key.code {
            KeyCode::Enter => Ok(ScreenFlow::Done(())),
            KeyCode::Esc => Ok(ScreenFlow::Cancel),
            _ => Ok(ScreenFlow::Continue),
        },
    )?;

    Ok(res.is_some())
}

fn verify_generated_mnemonic(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    expected_phrase: &str,
) -> anyhow::Result<bool> {
    let res = run_mnemonic_entry(
        terminal,
        VERIFY_GENERATED_MNEMONIC_SCREEN,
        |state| validate_generated_mnemonic_entry(state, expected_phrase),
    )?;

    Ok(res.is_some())
}

/// TUI screen for entering a 12-word mnemonic phrase.
/// Returns the validated phrase, or None if the user cancelled.
fn enter_mnemonic(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<Option<Zeroizing<String>>> {
    run_mnemonic_entry(
        terminal,
        RESTORE_MNEMONIC_SCREEN,
        validate_restore_mnemonic_entry,
    )
}

/// TUI screen for entering a new password with confirmation.
/// Returns the password, or None if the user cancelled.
fn enter_new_password(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<Option<String>> {
    struct State {
        password: Zeroizing<String>,
        confirm: Zeroizing<String>,
        on_confirm: bool,
        error: Option<String>,
    }

    run_screen(
        terminal,
        State {
            password: Zeroizing::new(String::new()),
            confirm: Zeroizing::new(String::new()),
            on_confirm: false,
            error: None,
        },
        PRE_WALLET_TICK_RATE,
        |frame, s| {
            render::render_new_password_screen(
                frame,
                s.password.len(),
                s.confirm.len(),
                s.on_confirm,
                s.error.as_deref(),
            );
        },
        |s, key| match key.code {
            KeyCode::Enter => {
                if !s.on_confirm {
                    s.on_confirm = true;
                    s.error = None;
                    Ok(ScreenFlow::Continue)
                } else if s.password != s.confirm {
                    s.error = Some("Passwords do not match.".into());
                    s.confirm.zeroize();
                    s.confirm.clear();
                    Ok(ScreenFlow::Continue)
                } else {
                    s.confirm.zeroize();
                    Ok(ScreenFlow::Done(std::mem::take(&mut *s.password)))
                }
            }
            KeyCode::Esc => {
                s.password.zeroize();
                s.confirm.zeroize();
                Ok(ScreenFlow::Cancel)
            }
            KeyCode::BackTab | KeyCode::Up => {
                if s.on_confirm {
                    s.on_confirm = false;
                    s.error = None;
                }
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Tab | KeyCode::Down => {
                if !s.on_confirm {
                    s.on_confirm = true;
                    s.error = None;
                }
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Char(c) => {
                if s.on_confirm {
                    s.confirm.push(c);
                } else {
                    s.password.push(c);
                }
                s.error = None;
                Ok(ScreenFlow::Continue)
            }
            KeyCode::Backspace => {
                if s.on_confirm {
                    s.confirm.pop();
                } else {
                    s.password.pop();
                }
                s.error = None;
                Ok(ScreenFlow::Continue)
            }
            _ => Ok(ScreenFlow::Continue),
        },
    )
}

/// Fetch balances for the current profile.
async fn fetch_balances(app: &mut App<'_>) {
    let idx = app.profile_idx;

    // Fetch moonlight balance
    match tokio::time::timeout(
        Duration::from_secs(5),
        app.wallet.get_moonlight_balance(idx),
    )
    .await
    {
        Ok(Ok(bal)) => {
            let entry = app.balances.entry(idx).or_default();
            entry.moonlight = Some(bal);
            app.connection.state = true;
        }
        Ok(Err(e)) => {
            app.connection.state = false;
            tracing::debug!("Failed to fetch moonlight balance: {e}");
        }
        Err(_) => {
            app.connection.state = false;
            tracing::debug!("Timed out while fetching moonlight balance");
        }
    }

    refresh_cached_phoenix(app);
}

async fn refresh_dashboard_data(app: &mut App<'_>, stake_err_context: &str) {
    fetch_balances(app).await;
    let stake_refresh_tip = app.current_profile_stake_refresh_tip_to_fetch();
    refresh_stake_info(app, stake_refresh_tip, stake_err_context).await;
}

fn refresh_cached_phoenix(app: &mut App<'_>) {
    let idx = app.profile_idx;
    match app.wallet.get_phoenix_balance_cached(idx) {
        Ok(bal) => {
            let entry = app.balances.entry(idx).or_default();
            entry.phoenix = Some(bal);
        }
        Err(e) => {
            tracing::debug!("Failed to fetch phoenix balance: {e}");
        }
    }
}

async fn refresh_stake_info(
    app: &mut App<'_>,
    stake_refresh_tip: Option<u64>,
    err_context: &str,
) {
    let attempted_at_tip = stake_refresh_tip.or(app.sync_block_height);

    if let Some(stake_refresh_tip) = stake_refresh_tip {
        app.note_current_profile_stake_refresh_attempt(stake_refresh_tip);
    }

    if let Err(err) = fetch_stake_info_with_timeout(
        app,
        STAKE_INFO_FETCH_TIMEOUT,
        attempted_at_tip,
    )
    .await
    {
        debug!("{err_context}: {err}");
    }
}

async fn fetch_stake_with_client(
    client: &RuesHttpClient,
    stake_pk: &BlsPublicKey,
) -> anyhow::Result<Option<StakeData>> {
    let bytes = client
        .contract_query::<_, 1024>(STAKE_CONTRACT, "get_stake", stake_pk)
        .await?;
    let stake_data = rkyv::check_archived_root::<Option<StakeData>>(&bytes)
        .map_err(|_| WalletError::Rkyv)?
        .deserialize(&mut rkyv::Infallible)
        .unwrap();

    Ok(stake_data)
}

async fn fetch_current_stake_owner(app: &App<'_>) -> Option<Address> {
    let stake_pk = app.current_profile().public_addr;
    match app.wallet.find_stake_owner_account(&stake_pk).await {
        Ok(owner) => Some(owner),
        Err(WalletError::NotStaked) => None,
        Err(err) => {
            debug!("Failed to fetch stake owner: {err}");
            None
        }
    }
}

fn stake_owner_label(profiles: &[Profile], owner: &Address) -> String {
    let label = match owner {
        Address::Public(public_key) => profiles
            .iter()
            .position(|profile| profile.public_addr == *public_key)
            .map_or_else(
                || "External profile".to_string(),
                |idx| format!("Profile {}", idx + 1),
            ),
        Address::Shielded(_) => "External profile".to_string(),
    };

    format!("{label} - {}", owner.preview())
}

/// Fetch stake info for the current profile.
/// Returns Err if the fetch failed (caller should show an error screen).
async fn fetch_stake_info(app: &mut App<'_>) -> anyhow::Result<()> {
    let idx = app.profile_idx;
    let attempted_at_tip = app.sync_block_height;
    let stake = app.wallet.stake_info(idx).await?;
    apply_stake_info(app, idx, stake, attempted_at_tip);
    Ok(())
}

async fn fetch_stake_info_with_timeout(
    app: &mut App<'_>,
    timeout: Duration,
    attempted_at_tip: Option<u64>,
) -> anyhow::Result<()> {
    let idx = app.profile_idx;
    let stake =
        match tokio::time::timeout(timeout, app.wallet.stake_info(idx)).await {
            Ok(Ok(stake)) => stake,
            Ok(Err(err)) => return Err(err.into()),
            Err(_) => anyhow::bail!("Timed out while fetching stake info"),
        };
    apply_stake_info(app, idx, stake, attempted_at_tip);
    Ok(())
}

fn apply_stake_info(
    app: &mut App<'_>,
    profile_idx: u8,
    stake: Option<dusk_core::stake::StakeData>,
    attempted_at_tip: Option<u64>,
) {
    use app::StakeState;

    let stake = match stake {
        Some(data) => StakeState::Loaded(data),
        None => StakeState::NoStake,
    };

    app.handle_async_result(AsyncResult::StakeUpdate {
        profile_idx,
        stake,
        attempted_at_tip,
    });
}

/// Execute a command and handle the result.
async fn execute_command(
    app: &mut App<'_>,
    cmd: crate::Command,
    tx: &mpsc::UnboundedSender<AsyncResult>,
) {
    let refresh_stake_info = matches!(
        &cmd,
        crate::Command::Stake { .. }
            | crate::Command::Unstake { .. }
            | crate::Command::ClaimRewards { .. }
            | crate::Command::Withdraw { .. }
    );
    let result = cmd.run(app.wallet, app.settings).await;

    match result {
        Ok(run_result) => {
            use crate::RunResult;
            match run_result {
                RunResult::Operation(result) => match result {
                    OperationResult::Tx(hash) => {
                        let tx_id = hex::encode(hash.to_bytes());
                        let _ = tx.send(AsyncResult::StatusMessage(
                            "Waiting for confirmation...".into(),
                        ));

                        if let Ok(gql) = GraphQL::new(
                            app.settings.state.to_string(),
                            app.settings.archiver.to_string(),
                            tui_status,
                        ) {
                            let _ = gql.wait_for(&tx_id).await;
                        }

                        app.handle_async_result(AsyncResult::Operation(
                            OperationResult::Tx(hash),
                        ));
                        fetch_balances(app).await;
                        if refresh_stake_info
                            && let Err(err) = fetch_stake_info(app).await
                        {
                            debug!(
                                "Failed to refresh stake info after confirmed transaction: {err}"
                            );
                        }
                    }
                    OperationResult::DeployTx { hash, contract_id } => {
                        let tx_id = hex::encode(hash.to_bytes());
                        let _ = tx.send(AsyncResult::StatusMessage(
                            "Waiting for confirmation...".into(),
                        ));

                        if let Ok(gql) = GraphQL::new(
                            app.settings.state.to_string(),
                            app.settings.archiver.to_string(),
                            tui_status,
                        ) {
                            let _ = gql.wait_for(&tx_id).await;
                        }

                        app.handle_async_result(AsyncResult::Operation(
                            OperationResult::DeployTx { hash, contract_id },
                        ));
                        fetch_balances(app).await;
                    }
                    OperationResult::ExportedKeys { pub_key, key_pair } => {
                        app.handle_async_result(AsyncResult::Operation(
                            OperationResult::ExportedKeys { pub_key, key_pair },
                        ));
                    }
                    OperationResult::Error { message } => {
                        app.handle_async_result(AsyncResult::Operation(
                            OperationResult::error(message),
                        ));
                    }
                },
                RunResult::Balance { balance, .. } => {
                    app.handle_async_result(AsyncResult::BalanceUpdate {
                        profile_idx: app.profile_idx,
                        balance,
                    });
                }
                RunResult::StakeInfo(data, _) => {
                    apply_stake_info(
                        app,
                        app.profile_idx,
                        Some(data),
                        app.sync_block_height,
                    );
                    app.screen = AppScreen::Dashboard;
                }
                RunResult::History(entries) => {
                    app.handle_async_result(AsyncResult::HistoryFetched(
                        entries,
                    ));
                }
                RunResult::ContractId(_)
                | RunResult::DriverDeployResult(_)
                | RunResult::Profile(_)
                | RunResult::Profiles(_)
                | RunResult::Create()
                | RunResult::Restore()
                | RunResult::Settings() => {
                    app.screen = AppScreen::Dashboard;
                }
            }
        }
        Err(err) => {
            app.handle_async_result(AsyncResult::Operation(
                OperationResult::error(err.to_string()),
            ));
        }
    }
}

/// Execute history fetch command.
async fn execute_history(app: &mut App<'_>) {
    app.screen = AppScreen::Executing {
        description: "Fetching transaction history...".into(),
    };

    let cmd = crate::Command::History {
        profile_idx: Some(app.profile_idx),
    };

    let result = cmd.run(app.wallet, app.settings).await;
    match result {
        Ok(crate::RunResult::History(entries)) => {
            app.history_selected = 0;
            app.screen = AppScreen::History { entries };
        }
        Err(err) => {
            app.handle_async_result(AsyncResult::Operation(
                OperationResult::error(err.to_string()),
            ));
        }
        _ => {
            app.screen = AppScreen::Dashboard;
        }
    }
}

/// Poll the background sync channel from the wallet.
fn poll_sync_channel(app: &mut App<'_>) {
    let messages: Vec<String> = app
        .wallet
        .state()
        .ok()
        .and_then(|state| state.sync_rx.as_ref())
        .map(|rx| {
            let mut msgs = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
            msgs
        })
        .unwrap_or_default();

    let mut sync_complete = false;
    for msg in messages {
        if msg.contains("Complete") || msg.contains("complete") {
            sync_complete = true;
        }
        app.handle_async_result(AsyncResult::SyncStatus(msg));
    }

    if sync_complete {
        refresh_cached_phoenix(app);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crossterm::event::KeyCode;
    use rusk_wallet::{Wallet, WalletPath, dat};
    use tempfile::tempdir;
    use zeroize::Zeroizing;

    use super::{
        GeneratedMnemonicVerification, MnemonicEntryState, WalletSetupAction,
        WalletSetupChoice, clear_wallet_cache, create_wallet_from_phrase,
        handle_mnemonic_char_input, verify_generated_mnemonic_input,
        wallet_setup_action_for_key,
    };
    use crate::{WalletFile, prompt};

    #[test]
    fn enter_defaults_to_create_wallet() {
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Enter),
            WalletSetupAction::Select(WalletSetupChoice::Create)
        );
    }

    #[test]
    fn create_key_selects_create_wallet() {
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Char('c')),
            WalletSetupAction::Select(WalletSetupChoice::Create)
        );
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Char('C')),
            WalletSetupAction::Select(WalletSetupChoice::Create)
        );
    }

    #[test]
    fn restore_key_selects_restore_wallet() {
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Char('r')),
            WalletSetupAction::Select(WalletSetupChoice::Restore)
        );
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Char('R')),
            WalletSetupAction::Select(WalletSetupChoice::Restore)
        );
    }

    #[test]
    fn quit_keys_cancel_wallet_setup() {
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Char('q')),
            WalletSetupAction::Cancel
        );
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Char('Q')),
            WalletSetupAction::Cancel
        );
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Esc),
            WalletSetupAction::Cancel
        );
    }

    #[test]
    fn unrelated_keys_leave_wallet_setup_running() {
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Left),
            WalletSetupAction::Continue
        );
        assert_eq!(
            wallet_setup_action_for_key(KeyCode::Char('x')),
            WalletSetupAction::Continue
        );
    }

    #[test]
    fn generated_mnemonic_verification_accepts_exact_match() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        assert_eq!(
            verify_generated_mnemonic_input(phrase, phrase),
            GeneratedMnemonicVerification::Match
        );
    }

    #[test]
    fn generated_mnemonic_verification_rejects_mismatch() {
        let expected = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let entered = "legal winner thank year wave sausage worth useful legal winner thank yellow";
        assert_eq!(
            verify_generated_mnemonic_input(entered, expected),
            GeneratedMnemonicVerification::Mismatch
        );
    }

    #[test]
    fn generated_mnemonic_verification_rejects_invalid_phrase() {
        let expected = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let entered = "not a valid mnemonic phrase";
        assert_eq!(
            verify_generated_mnemonic_input(entered, expected),
            GeneratedMnemonicVerification::Invalid
        );
    }

    #[test]
    fn mnemonic_entry_rejects_non_ascii_input() {
        let mut state = MnemonicEntryState {
            input: Zeroizing::new("abandon".into()),
            cursor: "abandon".len(),
            error: None,
        };

        handle_mnemonic_char_input(&mut state, 'é');

        assert_eq!(state.input.as_str(), "abandon");
        assert_eq!(state.cursor, "abandon".len());
        assert_eq!(
            state.error.as_deref(),
            Some("Mnemonic phrases only accept ASCII characters.")
        );
    }

    fn load_wallet_from_path(
        wallet_path: &WalletPath,
        password: &str,
    ) -> anyhow::Result<Wallet<WalletFile>> {
        let (file_version, salt_and_iv) =
            dat::read_file_version_and_salt_iv(wallet_path)?;
        let salt = salt_and_iv.as_ref().map(|si| &si.0);
        let key = prompt::derive_key(file_version, password, salt)?;

        Ok(Wallet::from_file(WalletFile {
            path: wallet_path.clone(),
            aes_key: key,
            salt: salt_and_iv.map(|si| si.0),
            iv: salt_and_iv.map(|si| si.1),
        })?)
    }

    #[test]
    fn replacing_existing_wallet_creates_backup_and_restarts_into_new_wallet()
    -> anyhow::Result<()> {
        const OLD_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        const NEW_PHRASE: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";
        const OLD_PASSWORD: &str = "old-password";
        const NEW_PASSWORD: &str = "new-password";

        let wallet_dir = tempdir()?;
        let mut wallet_path =
            WalletPath::from(wallet_dir.path().join("wallet.dat"));
        wallet_path.set_network_name(Some("test".to_string()));

        let old_wallet = create_wallet_from_phrase(
            &wallet_path,
            false,
            OLD_PHRASE,
            OLD_PASSWORD,
        )?;
        let old_public = old_wallet.default_address();
        let old_shielded = old_wallet.default_shielded_account();

        let cache_dir = wallet_path.cache_dir();
        fs::create_dir_all(&cache_dir)?;
        fs::write(cache_dir.join("cache.marker"), b"stale cache")?;
        assert!(
            cache_dir.exists(),
            "test setup should create a cache directory"
        );

        let replacement_wallet = create_wallet_from_phrase(
            &wallet_path,
            true,
            NEW_PHRASE,
            NEW_PASSWORD,
        )?;
        let new_public = replacement_wallet.default_address();
        let new_shielded = replacement_wallet.default_shielded_account();

        assert_ne!(old_public, new_public);
        assert_ne!(old_shielded, new_shielded);

        let mut backup_path = wallet_path.inner().to_path_buf();
        backup_path.set_extension("dat.old");
        assert!(
            backup_path.exists(),
            "replacing the wallet should create wallet.dat.old"
        );

        let backup_wallet_path = WalletPath::from(backup_path);
        let backup_wallet =
            load_wallet_from_path(&backup_wallet_path, OLD_PASSWORD)?;
        assert_eq!(backup_wallet.default_address(), old_public);
        assert_eq!(backup_wallet.default_shielded_account(), old_shielded);

        clear_wallet_cache(&wallet_path)?;
        assert!(
            !cache_dir.exists(),
            "restart path should clear the old wallet cache"
        );

        let reopened_wallet =
            load_wallet_from_path(&wallet_path, NEW_PASSWORD)?;

        assert_eq!(reopened_wallet.default_address(), new_public);
        assert_eq!(reopened_wallet.default_shielded_account(), new_shielded);
        assert_ne!(reopened_wallet.default_address(), old_public);
        assert!(
            backup_wallet_path.inner().exists(),
            "backup wallet should remain available after restart"
        );

        Ok(())
    }
}
