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

use bip39::{Language, Mnemonic};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use rusk_wallet::dat::{self, LATEST_VERSION};
use rusk_wallet::{GraphQL, Wallet, WalletPath};
use tokio::sync::mpsc;
use tracing::{debug, warn};
use zeroize::Zeroize;

use crate::WalletFile;
use crate::command::{gen_iv, gen_salt};
use crate::io::prompt;
use crate::settings::Settings;

pub use self::action::tui_status;
use self::action::{AsyncResult, clear_status_channel, init_status_channel};
use self::app::{App, AppAction, AppScreen, ConnectionStatus};

const TIP_HEIGHT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const TIP_HEIGHT_POLL_TIMEOUT: Duration = Duration::from_secs(4);

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
                // Back up old wallet, run restore flow, then re-enter
                backup_wallet(wallet_path);
                match restore_wallet_flow(&mut terminal, wallet_path)? {
                    Some(_) => continue,  // New wallet saved, restart
                    None => break Ok(()), // User cancelled
                }
            }
            Err(e) => break Err(e),
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    clear_status_channel();

    result
}

/// Back up existing wallet.dat to wallet.dat.old.
fn backup_wallet(wallet_path: &WalletPath) {
    let src = wallet_path.inner();
    if src.exists() {
        let mut dst = src.to_path_buf();
        dst.set_extension("dat.old");
        let _ = std::fs::copy(src, dst);
    }
}

/// Inner run function. Separated so terminal cleanup always happens.
async fn run_inner(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    wallet_path: &WalletPath,
    settings: &Settings,
) -> anyhow::Result<ExitReason> {
    // If no wallet file exists, offer to restore from mnemonic
    let mut wallet = if !wallet_path.inner().exists() {
        match restore_wallet_flow(terminal, wallet_path)? {
            Some(w) => w,
            None => return Ok(ExitReason::Quit), // User cancelled
        }
    } else {
        // Read wallet file metadata (doesn't need password)
        let (file_version, salt_and_iv) =
            dat::read_file_version_and_salt_iv(wallet_path)?;

        // Get password and load wallet (with retry on wrong password)
        let mut pwd_error: Option<&str> = None;
        let (wallet, password) = loop {
            let pwd = match &settings.password {
                Some(p) => p.clone(),
                None => match enter_password(terminal, pwd_error)? {
                    Some(p) => p,
                    None => return Ok(ExitReason::Quit), // User pressed Esc — quit
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
                    pwd_error = Some("Wrong password. Please try again.");
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        };

        // Handle old wallet file version (migration)
        let mut wallet = wallet;
        let wallet_fv = wallet.get_file_version()?;
        if wallet_fv.is_old() {
            let pwd_opt = Some(password.clone());
            crate::update_wallet_file(&mut wallet, &pwd_opt, wallet_fv)?;
        }

        // Zeroize the password — no longer needed
        let mut pwd = password;
        pwd.zeroize();

        wallet
    };

    // Phase 3: Connect
    let (tx, mut rx) = mpsc::unbounded_channel::<AsyncResult>();
    init_status_channel(tx.clone());

    if let Err(e) = wallet
        .connect_with_status(
            settings.state.as_str(),
            settings.prover.as_str(),
            settings.archiver.as_str(),
            tui_status,
        )
        .await
    {
        warn!("[OFFLINE MODE]: Unable to connect: {e}");
    }

    let connected: bool =
        tokio::time::timeout(Duration::from_secs(5), wallet.is_online())
            .await
            .unwrap_or_default();

    if connected && !run_startup_sync_gate(terminal, &wallet, &mut rx).await? {
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
                fetch_balances(&mut app).await;
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

        // Handle actions
        if let Some(action) = action {
            match action {
                AppAction::RefreshBalance => {
                    fetch_balances(&mut app).await;
                }
                AppAction::FetchHistory => {
                    execute_history(&mut app).await;
                }
                AppAction::FetchStakeInfo => {
                    match fetch_stake_info(&mut app).await {
                        Ok(()) => app.screen = AppScreen::StakeInfo,
                        Err(e) => app.handle_async_result(AsyncResult::Error(
                            e.to_string(),
                        )),
                    }
                }
                AppAction::ImportWallet => {
                    return Ok(ExitReason::ImportWallet);
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
                        warn!("[OFFLINE MODE]: Initial sync failed: {err}");
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
        AsyncResult::Error(msg) => {
            progress.push_status(format!("Error: {msg}"));
        }
        AsyncResult::BalanceUpdate { .. }
        | AsyncResult::StakeUpdate { .. }
        | AsyncResult::ChainTipHeight(_)
        | AsyncResult::TxComplete(_)
        | AsyncResult::DeployTxComplete(_, _)
        | AsyncResult::HistoryFetched(_)
        | AsyncResult::ExportedKeys(_, _) => {}
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
    let tick_rate = Duration::from_millis(100);

    struct State {
        password: String,
        error: Option<String>,
    }

    run_screen(
        terminal,
        State {
            password: String::new(),
            error: initial_error.map(String::from),
        },
        tick_rate,
        |frame, s| {
            render::render_password_screen(
                frame,
                s.password.len(),
                s.error.as_deref(),
            );
        },
        |s, key| match key.code {
            KeyCode::Enter => {
                Ok(ScreenFlow::Done(std::mem::take(&mut s.password)))
            }
            KeyCode::Esc => Ok(ScreenFlow::Cancel),
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

/// Full restore-from-mnemonic flow for when no wallet file exists.
/// Returns the created wallet, or None if the user cancelled.
fn restore_wallet_flow(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    wallet_path: &WalletPath,
) -> anyhow::Result<Option<Wallet<WalletFile>>> {
    // Step 1: Show welcome screen, wait for user to proceed or quit
    if !enter_welcome(terminal)? {
        return Ok(None);
    }

    // Step 2: Get mnemonic phrase (with BIP39 validation)
    let phrase = match enter_mnemonic(terminal)? {
        Some(p) => p,
        None => return Ok(None),
    };

    // Step 3: Get new password (with confirmation)
    let password = match enter_new_password(terminal)? {
        Some(p) => p,
        None => return Ok(None),
    };

    // Step 4: Create the wallet
    let salt = gen_salt();
    let iv = gen_iv();
    let file_version = dat::FileVersion::RuskBinaryFileFormat(LATEST_VERSION);
    let key = prompt::derive_key(file_version, &password, Some(&salt))?;

    let mut wallet: Wallet<WalletFile> = Wallet::new(phrase)?;
    wallet.save_to(WalletFile {
        path: wallet_path.clone(),
        aes_key: key,
        salt: Some(salt),
        iv: Some(iv),
    })?;

    let mut pwd = password;
    pwd.zeroize();

    Ok(Some(wallet))
}

/// Show the welcome screen when no wallet exists.
/// Returns true if the user wants to proceed, false to quit.
fn enter_welcome(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<bool> {
    let tick_rate = Duration::from_millis(100);

    let res = run_screen(
        terminal,
        (),
        tick_rate,
        |frame, _| render::render_welcome_screen(frame),
        |_, key| match key.code {
            KeyCode::Enter | KeyCode::Char('r') => Ok(ScreenFlow::Done(())),
            KeyCode::Esc | KeyCode::Char('q') => Ok(ScreenFlow::Cancel),
            _ => Ok(ScreenFlow::Continue),
        },
    )?;

    Ok(res.is_some())
}

/// TUI screen for entering a 12-word mnemonic phrase.
/// Returns the validated phrase, or None if the user cancelled.
fn enter_mnemonic(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<Option<String>> {
    let tick_rate = Duration::from_millis(100);

    struct State {
        input: String,
        cursor: usize,
        error: Option<String>,
    }

    run_screen(
        terminal,
        State {
            input: String::new(),
            cursor: 0,
            error: None,
        },
        tick_rate,
        |frame, s| {
            render::render_mnemonic_screen(
                frame,
                &s.input,
                s.cursor,
                s.error.as_deref(),
            );
        },
        |s, key| match key.code {
            KeyCode::Enter => {
                let trimmed = s.input.trim().to_string();
                match Mnemonic::from_phrase(&trimmed, Language::English) {
                    Ok(mnem) => Ok(ScreenFlow::Done(mnem.to_string())),
                    Err(_) => {
                        s.error = Some(
                            "Invalid mnemonic. Enter 12 valid BIP39 words \
                             separated by spaces."
                                .into(),
                        );
                        Ok(ScreenFlow::Continue)
                    }
                }
            }
            KeyCode::Esc => Ok(ScreenFlow::Cancel),
            KeyCode::Char(c) => {
                s.input.insert(s.cursor, c);
                s.cursor += 1;
                s.error = None;
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

/// TUI screen for entering a new password with confirmation.
/// Returns the password, or None if the user cancelled.
fn enter_new_password(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<Option<String>> {
    let tick_rate = Duration::from_millis(100);

    struct State {
        password: String,
        confirm: String,
        on_confirm: bool,
        error: Option<String>,
    }

    run_screen(
        terminal,
        State {
            password: String::new(),
            confirm: String::new(),
            on_confirm: false,
            error: None,
        },
        tick_rate,
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
                    s.confirm.clear();
                    Ok(ScreenFlow::Continue)
                } else {
                    Ok(ScreenFlow::Done(std::mem::take(&mut s.password)))
                }
            }
            KeyCode::Esc => Ok(ScreenFlow::Cancel),
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

/// Fetch stake info for the current profile.
/// Returns Err if the fetch failed (caller should show an error screen).
async fn fetch_stake_info(app: &mut App<'_>) -> anyhow::Result<()> {
    use app::StakeState;
    let idx = app.profile_idx;
    match app.wallet.stake_info(idx).await {
        Ok(Some(data)) => {
            app.stake_info.insert(idx, StakeState::Loaded(data));
        }
        Ok(None) => {
            app.stake_info.insert(idx, StakeState::NoStake);
        }
        Err(e) => {
            return Err(e.into());
        }
    }
    Ok(())
}

/// Execute a command and handle the result.
async fn execute_command(
    app: &mut App<'_>,
    cmd: crate::Command,
    tx: &mpsc::UnboundedSender<AsyncResult>,
) {
    let result = cmd.run(app.wallet, app.settings).await;

    match result {
        Ok(run_result) => {
            use crate::RunResult;
            match run_result {
                RunResult::Tx(hash) => {
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

                    app.handle_async_result(AsyncResult::TxComplete(hash));
                    fetch_balances(app).await;
                }
                RunResult::DeployTx(hash, contract_id) => {
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

                    app.handle_async_result(AsyncResult::DeployTxComplete(
                        hash,
                        contract_id,
                    ));
                    fetch_balances(app).await;
                }
                RunResult::PhoenixBalance(balance, _) => {
                    app.handle_async_result(AsyncResult::BalanceUpdate {
                        profile_idx: app.profile_idx,
                        phoenix: Some(balance),
                        moonlight: None,
                    });
                }
                RunResult::MoonlightBalance(balance) => {
                    app.handle_async_result(AsyncResult::BalanceUpdate {
                        profile_idx: app.profile_idx,
                        phoenix: None,
                        moonlight: Some(balance),
                    });
                }
                RunResult::StakeInfo(data, _) => {
                    app.handle_async_result(AsyncResult::StakeUpdate {
                        profile_idx: app.profile_idx,
                        stake: app::StakeState::Loaded(data),
                    });
                    app.screen = AppScreen::Dashboard;
                }
                RunResult::ExportedKeys(pub_key, key_pair) => {
                    app.handle_async_result(AsyncResult::ExportedKeys(
                        pub_key, key_pair,
                    ));
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
            app.handle_async_result(AsyncResult::Error(err.to_string()));
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
            app.handle_async_result(AsyncResult::Error(err.to_string()));
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
