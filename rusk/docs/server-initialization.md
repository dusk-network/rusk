# Current Feature Implementation Plan

## Feature Description

Implement Stage 1, Task 4.1: **HTTPS Server Initialization using `axum`**. This involves setting up an `axum` web server configured to listen for HTTPS connections (using `axum-server` and `rustls`), loading TLS certificates specified in `JsonRpcConfig`, adding basic routes (`/health`), and preparing the groundwork for integrating `jsonrpsee` as the JSON-RPC request handler on the `/rpc` route (integration details in Task 4.5). Middleware and RPC module integration will be handled in subsequent tasks (4.2, 4.3).

**Prerequisite:** The `AppState` required by the `run_server` function must be created *before* `run_server` is called. This creation happens during the main Rusk node startup sequence (likely within `rusk::Builder::build_and_run` or related functions):

1. The `RuskNode` builder initializes the core node components: Database (`node::database::rocksdb::Backend`), Archive (`node::archive::Archive`), Network (`node::Network`), and VM (`node::vm::VMExecution`).
2. The builder obtains handles (`Arc<RwLock<...>>`) to these initialized components.
3. The builder creates the concrete adapters using clones of the handles:
   - `RuskDbAdapter::new(db_handle.clone())` (feature `chain`)
   - `RuskArchiveAdapter::new(archive_handle.clone())` (feature `archive`)
   - `RuskNetworkAdapter::new(network_handle.clone())` (feature `chain`)
   - `RuskVmAdapter::new(vm_handle.clone())` (feature `chain`)
4. The builder creates the `AppState` instance, injecting the configuration and *all* required adapters (wrapped in `Arc<dyn ...>`) and other components (SubscriptionManager, MetricsCollector, ManualRateLimiters).
5. This fully constructed `AppState` (wrapped in `Arc`) is then passed to `run_server`.

The implementation details of this `AppState` creation sequence within the `Builder` are detailed in the "Modify `RuskNodeBuilder::build_and_run`..." section below, but understanding this prerequisite flow is essential.

## Detailed Implementation Plan in form of tasks

- [ ] **Add Dependencies:** Add `axum-0.8.3`, `axum-server-0.7.2` (with `tls-rustls` feature), `tokio-rustls`, `rustls`, `rustls-pemfile` to `Cargo.toml` and `rusk/Cargo.toml` dependencies.
- [ ] **Define Server Execution Function:** Create `run_server` function in `rusk::lib::jsonrpc::server`, accepting `AppState` (assuming `AppState` is fully constructed as described in the prerequisite). This function orchestrates the `axum` server setup and launch.
- [ ] **Load TLS Configuration (in `run_server`):**
  - [ ] Read `cert` and `key` paths from `AppState.config().http`. Return `Error::Config(ConfigError::FileRead)` if paths are specified but files cannot be read.
  - [ ] If paths are provided, use `rustls_pemfile` to load certificate chains and the private key. Map I/O errors to `Error::Config(ConfigError::FileRead)`. Map parsing errors to `Error::Config(ConfigError::Validation("Invalid TLS certificate or key format"))`.
  - [ ] **Add Logging (TLS Configuration):**
    - [ ] Log (info) whether TLS configuration (cert/key paths) is provided.
    - [ ] Log (info) successful loading of TLS certificate and key.
    - [ ] Log (error) if reading TLS files fails (I/O error).
    - [ ] Log (error) if parsing TLS files fails (format error).
  - [ ] Create a `rustls::ServerConfig` using `ServerConfig::builder().with_no_client_auth().with_single_cert()`. Handle potential errors.
  - [ ] Create `axum_server::tls_rustls::RustlsConfig` from the `rustls::ServerConfig`. Consider `.fallback()` if HTTP should be allowed when TLS isn't configured.
- [ ] **Define Basic `axum` Router (in `run_server`):**
  - [ ] Create an `axum::Router`. Add `axum::extract::State` layer for `Arc<AppState>`.
  - [ ] Add a basic `GET /health` route returning `200 OK` (e.g., `axum::routing::get(|| async { "OK" })`).
  - [ ] **(Placeholder)** Define the `/rpc` route using `axum::routing::any_service(...)`. The service itself will be created in Task 4.5. Add a `// TODO (Task 4.5): Create jsonrpsee tower service and integrate here.` comment.
- [ ] **Bind and Serve (in `run_server`):**
  - [ ] Get the `bind_address` from `AppState.config().http`.
  - [ ] Create a `tokio::signal::ctrl_c` future for graceful shutdown.
  - [ ] **Add Logging (Pre-Bind Info):**
    - [ ] Log (info) the target bind address.
    - [ ] Log (info) whether TLS is configured and HTTPS will be attempted.
  - [ ] If TLS is configured (`RustlsConfig` was created):
    - [ ] Use `axum_server::bind_rustls(bind_address, rustls_config)`.
    - [ ] Serve the router using `.serve(router.into_make_service_with_connect_info::<SocketAddr>())`.
    - [ ] Add `.with_graceful_shutdown()` using the signal future.
    - [ ] **Add Logging (HTTPS Bind Success):** Log (info) the actual listening address (HTTPS) upon successful binding.
  - [ ] Else (TLS is not configured):
    - [ ] Use `axum::serve(TcpListener::bind(bind_address).await?, router.into_make_service_with_connect_info::<SocketAddr>())`.
    - [ ] Add `.with_graceful_shutdown()` using the signal future.
    - [ ] **Add Logging (HTTP Bind Success):** Log (info) the actual listening address (HTTP) upon successful binding.
  - [ ] Handle potential binding/serving errors, mapping them to `Error::Transport` or `Error::Internal`.
  - [ ] **Add Logging (Errors & Shutdown):**
    - [ ] Log (error) detailed error message if binding fails (within error handling).
    - [ ] Log (info) when the graceful shutdown signal is received (within shutdown logic).
    - [ ] Log (info) when the server shutdown completes successfully (after serve call).
    - [ ] Log (error) detailed error message if serving fails (within error handling).
- [ ] **Add Documentation:** Document `run_server`, explaining the `axum`/`rustls` setup, routing, graceful shutdown, and the placeholder for RPC integration.
- [ ] **Implement Basic Tests:**
  - [ ] Test server starts and `/health` endpoint is reachable over HTTPS (if TLS configured) or HTTP (if not configured) using a suitable client (`reqwest`).
  - [ ] Test TLS configuration loading failures (invalid paths, bad files).

## Modify `RuskNodeBuilder::build_and_run` for JSON-RPC Server Launch

These tasks modify `rusk/src/lib/builder/node.rs` to create the `AppState` (including all necessary adapters) and launch the new `axum`-based JSON-RPC server alongside the existing WebSocket server.

- [ ] **Load `JsonRpcConfig`:**
  - **Where:** In `RuskNodeBuilder::build_and_run`, before initializing node components like `Rusk::new` or fetching handles.
  - **How:**
    - Call `rusk::jsonrpc::config::JsonRpcConfig::load_default()`.
    - Handle `Ok(config)`: Store the loaded config. Log success. Check if the config indicates the server should be enabled (e.g., `http.enabled` flag or non-zero port) and store it in an `Option<JsonRpcConfig>`. Log warning and set to `None` if disabled.
    - Handle `Err(e)`: Log the error. Set the config variable to `None`.
- [ ] **Conditionally Initialize JSON-RPC Components:**
  - **Where:** After the core `Rusk` node and its components (Database, Archive, Network, VM Handler) are initialized and their handles (`Arc<RwLock<...>>`) are available, likely after `Rusk::new` or equivalent setup.
  - **How:** Add an `if let Some(config) = json_rpc_config { ... }` block. Only proceed if config was loaded successfully and the server is enabled.
- [ ] **Get Component Handles (Inside conditional block):**
  - **Where:** Inside the `if let Some(config) = ...` block.
  - **How:** Obtain `Arc<RwLock<...>>` handles for the initialized:
    - Database backend (`node::database::rocksdb::Backend`).
    - Archive component (`node::archive::Archive`).
    - Network component (`Kadcast` implementing `node::Network`).
    - VM Handler component (implementing `node::vm::VMExecution`).
    - Ensure these handles are readily available and cloneable from the main `RuskNode` or builder state.
- [ ] **Create `RuskDbAdapter` (Inside conditional block, gated by `chain` feature):**
  - **Where:** Inside the `if let Some(config) = ...` block.
  - **How:** Use `#[cfg(feature = "chain")]`. Call `rusk::jsonrpc::infrastructure::db::RuskDbAdapter::new(db_handle.clone())`. Wrap the result in `Arc`. Log creation.
- [ ] **Create `RuskArchiveAdapter` (Inside conditional block, gated by `archive` feature):**
  - **Where:** Inside the `if let Some(config) = ...` block.
  - **How:** Use `#[cfg(feature = "archive")]`. Call `rusk::jsonrpc::infrastructure::archive::RuskArchiveAdapter::new(archive_handle.clone())`. Wrap the result in `Arc`. Log creation.
- [ ] **Create `RuskNetworkAdapter` (Inside conditional block, gated by `chain` feature):**
  - **Where:** Inside the `if let Some(config) = ...` block.
  - **How:** Use `#[cfg(feature = "chain")]`. Call `rusk::jsonrpc::infrastructure::network::RuskNetworkAdapter::new(network_handle.clone())`. Wrap the result in `Arc`. Log creation.
- [ ] **Create `RuskVmAdapter` (Inside conditional block, gated by `chain` feature):**
  - **Where:** Inside the `if let Some(config) = ...` block.
  - **How:** Use `#[cfg(feature = "chain")]`. Call `rusk::jsonrpc::infrastructure::vm::RuskVmAdapter::new(vm_handle.clone())`. Wrap the result in `Arc`. Log creation.
- [ ] **Create Other `AppState` Components (Inside conditional block):**
  - **Where:** Inside the `if let Some(config) = ...` block.
  - **How:**
    - Create `SubscriptionManager::default()`. Log creation.
    - Create `MetricsCollector::default()`. Log creation.
    - Create `Arc::new(config.rate_limit.clone())` for rate limit config.
    - Create `ManualRateLimiters::new(...)` using the rate limit config Arc. Handle potential errors (log error, decide whether to panic or prevent server start). Log creation.
- [ ] **Create `AppState` (Inside conditional block, gated by `chain` and potentially `archive` features):**
  - **Where:** Inside the `if let Some(config) = ...` block, after all adapters and components are created.
  - **How:**
    - Use `#[cfg(all(feature = "chain", feature = "archive"))]` (Adjust based on whether all adapters are mandatory; assume they are for full functionality).
    - Gather all created components: `config.clone()`, `db_adapter`, `archive_adapter`, `network_adapter`, `vm_adapter`, `subscription_manager`, `metrics_collector`, `manual_rate_limiters`.
    - Call `rusk::jsonrpc::infrastructure::state::AppState::new(...)`, passing all the gathered components.
    - Wrap the result in `Arc`. Log creation.
    - Add an `#else` block for the `cfg` to handle cases where required features are missing (e.g., compile error or log warning and skip server start).
- [ ] **Spawn `run_server` Task (Inside conditional block, gated appropriately):**
  - **Where:** Inside the `if let Some(config) = ...` block and potentially inside the `#[cfg(...)]` block where `AppState` was successfully created.
  - **How:** Use `tokio::spawn(async move { ... })`. Inside the task, call `rusk::jsonrpc::server::run_server(app_state).await`. Add error handling (`if let Err(e) = ...`) to log server failures. Log spawn initiation and graceful shutdown/failure.
- [ ] **Preserve Existing `HttpServer`:**
  - **Where:** The existing block `if let Some(http) = self.http { ... }`.
  - **How:** Leave this block unmodified for now. Add a `// TODO:` comment explaining that this server currently handles WebSockets and needs to be consolidated with the new `axum` server in a later stage.

## Files Involved (reference, documentation, implementation)

- **`rusk/docs/current-feature-plan.md`**: (This plan)
- **`rusk/docs/plan.md`**: (Reference for Stage 1, Tasks 4.1-4.5)
- **`rusk/docs/jsonrpsee_to_tower_service.md`**: (New documentation on integration)
- **`rusk/src/lib/jsonrpc/config.rs`**: (To access HTTP and TLS configuration)
- **`rusk/src/lib/jsonrpc/server.rs`**: (Location for the `run_server` function)
- **`rusk/src/lib/jsonrpc/error.rs`**: (For mapping errors)
- **`rusk/src/lib/jsonrpc/infrastructure/state.rs`**: (Definition of `AppState`)
- **`rusk/tests/jsonrpc/server.rs`**: (Tests for `run_server` basic functionality)
- **`Cargo.toml`**: (Add new dependencies)
- **`rusk/Cargo.toml`**: (Add new dependencies)

## Other Notes

- This task focuses on setting up the `axum` web server foundation, including TLS.
- The actual creation and integration of the `jsonrpsee` RPC service handler is deferred to **Task 4.4** and **Task 4.5**.
- Global middleware (CORS, Governor) application on the `axum` router is deferred to **Task 4.2**.
- This approach uses `axum` to handle TLS termination directly within the Rust process.
