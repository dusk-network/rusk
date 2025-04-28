// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Manages the startup and execution of the JSON-RPC server (Axum/HTTPS).

use crate::jsonrpc::config::{ConfigError, HttpServerConfig};
use crate::jsonrpc::error::Error;
use crate::jsonrpc::infrastructure::state::AppState;
use axum::{
    routing::{any_service, get},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use axum_server::Handle;
use http_body_util::{BodyExt, Empty};
use hyper::Response;
use rustls::pki_types::PrivateKeyDer;
use rustls_pemfile::certs;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info};

// Imports for Governor
use std::num::NonZeroU32;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor,
    GovernorLayer,
};

// Imports for CORS
use tower_http::cors::{Any, CorsLayer};

/// Loads TLS configuration from the specified certificate and key files.
///
/// Reads the certificate chain and private key from the paths provided in
/// `HttpServerConfig`.
///
/// # Arguments
///
/// * `tls_config` - The HTTP server configuration containing optional cert/key
///   paths.
///
/// # Returns
///
/// Returns `Ok(Some(RustlsConfig))` if TLS paths are provided and loaded
/// successfully. Returns `Ok(None)` if TLS paths are not configured. Returns
/// `Err(Error)` if paths are provided but files cannot be read or parsed.
async fn load_tls_config(
    tls_config: &HttpServerConfig,
) -> Result<Option<RustlsConfig>, Error> {
    let (cert_path, key_path) = match (&tls_config.cert, &tls_config.key) {
        (Some(cert), Some(key)) => (cert, key),
        (None, None) => {
            info!("TLS certificate and key paths not provided. Starting server without HTTPS.");
            return Ok(None);
        }
        (Some(_), None) => {
            let err_msg =
                "TLS certificate path provided, but key path is missing";
            error!(error = err_msg);
            return Err(Error::Config(ConfigError::Validation(err_msg.into())));
        }
        (None, Some(_)) => {
            let err_msg =
                "TLS key path provided, but certificate path is missing";
            error!(error = err_msg);
            return Err(Error::Config(ConfigError::Validation(err_msg.into())));
        }
    };

    info!(cert_path = %cert_path.display(), key_path = %key_path.display(), "Attempting to load TLS certificate and key");

    let cert_file = File::open(cert_path).map_err(|e| {
        error!(path = %cert_path.display(), error = %e, "Failed to open TLS certificate file");
        Error::Config(e.into())
    })?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_chain = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            error!(path = %cert_path.display(), error = %e, "Failed to read/parse TLS certificate file (IO Error)");
            Error::Config(e.into())
        })?;

    if cert_chain.is_empty() {
        error!(path = %cert_path.display(), "No valid PEM certificates found in file");
        return Err(Error::Config(ConfigError::Validation(
            format!(
                "No valid PEM certificates found in {}",
                cert_path.display()
            )
            .into(),
        )));
    }

    let key_file = File::open(key_path).map_err(|e| {
        error!(path = %key_path.display(), error = %e, "Failed to open TLS private key file");
        Error::Config(e.into())
    })?;
    let mut key_reader = BufReader::new(key_file);

    let key = loop {
        match rustls_pemfile::read_one(&mut key_reader)
            .map_err(|io_err| Error::Config(ConfigError::FileRead(io_err)))?
        {
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => {
                break PrivateKeyDer::Pkcs8(key)
            }
            Some(rustls_pemfile::Item::Sec1Key(key)) => {
                break PrivateKeyDer::Sec1(key)
            }
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => {
                break PrivateKeyDer::Pkcs1(key)
            }
            Some(_) => continue,
            None => {
                error!(path = %key_path.display(), "No valid PEM private keys found in file");
                return Err(Error::Config(ConfigError::Validation(
                    format!(
                        "No valid PEM private keys found in {}",
                        key_path.display()
                    )
                    .into(),
                )));
            }
        }
    };

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| {
            error!(error = %e, "Failed to create rustls ServerConfig");
            Error::Internal(format!("Failed to configure rustls: {}", e))
        })?;

    info!(
        "Successfully loaded TLS certificate and key. HTTPS will be enabled."
    );

    let rustls_config = RustlsConfig::from_config(Arc::new(server_config));

    Ok(Some(rustls_config))
}

/// Spawns a task to listen for CTRL+C and trigger graceful shutdown via the
/// provided handle.
fn spawn_graceful_shutdown_handler(handle: Handle) {
    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Received graceful shutdown signal (CTRL+C). Triggering shutdown...");
        let shutdown_timeout = Duration::from_secs(30);
        handle.graceful_shutdown(Some(shutdown_timeout));
        info!("Graceful shutdown initiated (timeout: {:?}). Waiting for connections to close...", shutdown_timeout);
    });
}

/// Runs the JSON-RPC server using Axum.
///
/// This function initializes and runs the Axum web server, configured for
/// either HTTP or HTTPS based on the provided `JsonRpcConfig`. It sets up
/// basic routes, TLS (if configured), graceful shutdown handling, and prepares
/// for integrating the `jsonrpsee` service.
///
/// ## Middleware
///
/// The server applies the following middleware layers:
/// - **CORS:** A permissive CORS layer (`tower_http::cors::CorsLayer`) allowing
///   requests from any origin, method, or header.
/// - **Rate Limiting:** Global rate limiting per IP address using
///   `tower_governor::GovernorLayer`, configured by
///   `JsonRpcConfig.rate_limit.default_limit`. This is applied only if rate
///   limiting is enabled in the config.
/// - **State:** The shared `AppState` is added as an Axum state layer for
///   access in handlers.
///
/// # Arguments
///
/// * `app_state` - The shared application state containing configuration,
///   adapters, and other necessary components.
///
/// # Returns
///
/// Returns `Ok(())` if the server runs and shuts down gracefully. Returns an
/// `Err(Error)` if there's an issue during startup (e.g., binding, TLS config)
/// or runtime.
pub async fn run_server(app_state: Arc<AppState>) -> Result<(), Error> {
    // 1. Load TLS configuration
    let rustls_config = load_tls_config(&app_state.config().http).await?;

    // 2. Define Axum router
    let mut router = Router::new()
        // Basic health check endpoint
        .route("/health", get(|| async { "OK" }))
        // Placeholder for the main JSON-RPC endpoint
        .route(
            "/rpc",
            any_service(tower::service_fn(|_req| async {
                Ok::<_, std::convert::Infallible>(
                    Response::builder()
                        .status(hyper::StatusCode::NOT_IMPLEMENTED)
                        .body(Empty::<axum::body::Bytes>::new().boxed_unsync())
                        .unwrap(),
                )
            })),
        )
        // Add the AppState extractor layer
        .with_state(app_state.clone());

    // --- Add Governor Rate Limiting Middleware --- START ---
    if app_state.config().rate_limit.enabled {
        info!("Rate limiting enabled. Applying tower-governor middleware...");

        let default_limit = &app_state.config().rate_limit.default_limit;
        let requests = default_limit.requests as u32; // Governor uses u32
        let window = default_limit.window;

        // Ensure requests and window are non-zero to avoid governor panics
        let requests_non_zero =
            NonZeroU32::new(requests).unwrap_or_else(|| {
                error!(
                    requests,
                    "Rate limit requests must be non-zero. Defaulting to 1."
                );
                NonZeroU32::new(1).unwrap()
            });

        if window == Duration::ZERO {
            error!(window=?window, "Rate limit window must be non-zero. Panicking.");
            panic!("Invalid rate limit configuration: window cannot be zero");
        }

        let governor_config = GovernorConfigBuilder::default()
            .per(window) // Use per-window duration
            .burst_size(requests_non_zero) // Set burst size
            .key_extractor(PeerIpKeyExtractor) // Limit per IP
            .finish()
            .map_err(|e| {
                Error::Internal(format!(
                    "Failed to create Governor config: {}",
                    e
                ))
            })?; // Handle potential config error

        // Note: GovernorLayer requires the config to be 'static.
        // Leaking or using Arc might be needed if config needs mutation,
        // but here we build it once.
        // Using Box::leak as shown in tower-governor examples for simplicity.
        let governor_config_leaked = Box::leak(Box::new(governor_config));

        router = router.layer(GovernorLayer {
            config: governor_config_leaked,
        });

        info!("tower-governor middleware applied.");
    } else {
        info!("Rate limiting disabled globally. Skipping tower-governor middleware.");
    }
    // --- Add Governor Rate Limiting Middleware --- END ---

    // --- Add CORS Middleware --- START ---
    info!("Applying permissive CORS middleware...");
    let cors_layer = CorsLayer::new()
        // Allow requests from any origin
        .allow_origin(Any)
        // Allow common methods
        .allow_methods(Any)
        // Allow all headers
        .allow_headers(Any);

    router = router.layer(cors_layer);
    info!("CORS middleware applied.");
    // --- Add CORS Middleware --- END ---

    // 3. Prepare for binding and serving
    let bind_address = app_state.config().http.bind_address;
    let handle = Handle::new(); // Create the handle

    // Spawn the handler task BEFORE starting the server
    spawn_graceful_shutdown_handler(handle.clone());

    info!(%bind_address, tls_enabled = rustls_config.is_some(), "Attempting to bind server...");

    // 4. Bind and serve using the handle
    let serve_result = if let Some(tls) = rustls_config {
        // --- Serve HTTPS ---
        info!("Binding HTTPS server to {}...", bind_address);
        axum_server::bind_rustls(bind_address, tls)
            .handle(handle) // Pass the handle
            .serve(router.into_make_service_with_connect_info::<SocketAddr>())
            .await // Await server termination
            .map_err(|e| {
                error!(address = %bind_address, error = %e, "HTTPS server failed during operation");
                Error::Transport(format!("HTTPS Server error: {}", e))
            })
    } else {
        // --- Serve HTTP ---
        info!("Binding HTTP server to {}...", bind_address);
        // For HTTP, we need to bind the listener first to get the actual
        // address
        let listener = TcpListener::bind(bind_address).await.map_err(|e| {
            error!(address = %bind_address, error = %e, "Failed to bind HTTP listener");
            Error::Transport(format!("Failed to bind to {}: {}", bind_address, e))
        })?;
        let actual_addr = listener.local_addr().map_err(|e| {
            error!(error = %e, "Failed to get local address after binding");
            Error::Internal(format!("Failed to get local address: {}", e))
        })?;
        info!(address = %actual_addr, "HTTP server listening");

        // Use axum_server::from_tcp to apply the handle
        // Convert tokio::net::TcpListener to std::net::TcpListener
        let std_listener = listener.into_std().map_err(|e| {
            error!(address = %actual_addr, error = %e, "Failed to convert listener to std");
            Error::Internal(format!("Listener conversion error: {}", e))
        })?;

        axum_server::from_tcp(std_listener) // Pass the std listener
            .handle(handle) // Pass the handle
            .serve(router.into_make_service_with_connect_info::<SocketAddr>())
            .await // Await server termination
            .map_err(|e| {
                error!(address = %actual_addr, error = %e, "HTTP server failed during operation");
                Error::Transport(format!("HTTP Server error: {}", e))
            })
    };

    match serve_result {
        Ok(_) => {
            info!("Server shutdown sequence finished.");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Server terminated with error");
            Err(e)
        }
    }
}

// TODO Add basic tests for server startup and `/health` endpoint
// in `rusk/tests/jsonrpc/server.rs`.
