// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Manages the startup and execution of the JSON-RPC server (Axum/HTTPS).

use crate::jsonrpc::config::{ConfigError, HttpServerConfig};
use crate::jsonrpc::error::Error;
use crate::jsonrpc::infrastructure::state::AppState;
use axum::routing::get;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use axum_server::Handle;
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

use crate::jsonrpc::rpc_methods::{RuskInfoRpcImpl, RuskInfoRpcServer};
use axum::{
    extract::{FromRequest, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use jsonrpsee::server::RpcModule;

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
        return Err(Error::Config(ConfigError::Validation(format!(
            "No valid PEM certificates found in {}",
            cert_path.display()
        ))));
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
                return Err(Error::Config(ConfigError::Validation(format!(
                    "No valid PEM private keys found in {}",
                    key_path.display()
                ))));
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
/// provided axum handle.
fn spawn_graceful_shutdown_handler(axum_handle: Handle) {
    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!(
            "Received graceful shutdown signal (CTRL+C). Triggering shutdown..."
        );

        // Trigger axum server graceful shutdown
        let shutdown_timeout = Duration::from_secs(30);
        axum_handle.graceful_shutdown(Some(shutdown_timeout));
        info!(
            "Axum graceful shutdown initiated (timeout: {:?}). Waiting for connections to close...",
            shutdown_timeout
        );
    });
}

/// Custom extractor to get raw request body bytes.
struct RawBytes(Vec<u8>);

impl<S> FromRequest<S> for RawBytes
where
    S: Send + Sync,
{
    type Rejection = Response;

    fn from_request(
        req: Request,
        _state: &S,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Self, Self::Rejection>>
                + Send,
        >,
    > {
        Box::pin(async move {
            let body = req.into_body();
            let bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(
                |err| {
                    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
                        .into_response()
                },
            )?;
            Ok(RawBytes(bytes.into()))
        })
    }
}

/// Axum handler for JSON-RPC requests.
///
/// This function manually extracts the raw JSON-RPC request body, calls the
/// `jsonrpsee` RpcModule to process it, and converts the response back into an
/// Axum response.
async fn rpc_handler(
    State(module): State<RpcModule<Arc<AppState>>>, /* Access RpcModule from
                                                     * state */
    RawBytes(raw_body): RawBytes, // Use custom extractor for raw body
) -> Response {
    const MAX_RESPONSE_SIZE: usize = 1024 * 1024; // Example limit: 1MB

    let body_str = match std::str::from_utf8(&raw_body) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to decode request body as UTF-8");
            // Consider returning a JSON-RPC error response here
            return (StatusCode::BAD_REQUEST, "Invalid UTF-8 in request body")
                .into_response();
        }
    };
    // Process the raw request using jsonrpsee
    let (response_result, _stream) =
        match module.raw_json_request(body_str, MAX_RESPONSE_SIZE).await {
            Ok(response) => response,
            Err(e) => {
                error!(error = %e, "Failed to process JSON-RPC request");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to process RPC request",
                )
                    .into_response();
            }
        };

    // raw_json_request returns the response string and a stream (for
    // subscriptions) We ignore the stream for now.

    // Convert the response string into an Axum response
    // Ensure correct Content-Type header
    Response::builder()
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(response_result))
        .unwrap_or_else(|_| {
            // Fallback if building response fails
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to construct RPC response",
            )
                .into_response()
        })
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
/// - **CORS:** Configurable CORS layer via [`JsonRpcConfig::build_cors_layer`].
/// - **Rate Limiting:** Global rate limiting per IP address using
///   `tower_governor::GovernorLayer`, configured by
///   `JsonRpcConfig.rate_limit.default_limit`. This is applied only if rate
///   limiting is enabled in the config.
///
/// ## RPC Handling
///
/// Requests to the `/rpc` path are routed to a `jsonrpsee` server instance
/// which handles JSON-RPC method dispatch based on the `RpcModule` configured
/// with methods from [`crate::jsonrpc::rpc_methods`].
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

    // Instantiate the RPC implementation
    let rpc_impl = RuskInfoRpcImpl::new(app_state.clone());
    // Create a new RpcModule with AppState context
    let mut rpc_module = RpcModule::new(app_state.clone());
    // Merge the implementation
    rpc_module.merge(rpc_impl.into_rpc()).map_err(|e| {
        Error::Internal(format!("Failed to merge RuskInfoRpc methods: {}", e))
    })?;
    info!("JSON-RPC module prepared and methods merged.");

    // 2. Define Axum router
    let router = Router::new()
        // Basic health check endpoint
        .route("/health", get(|| async { "OK" }))
        // Route /rpc requests to our manual handler
        .route("/rpc", post(rpc_handler)) // Use post() for RPC handler
        // Add the RpcModule as state for the handler
        .with_state(rpc_module);

    // 3. Apply rate limiting if configured using tower-governor
    let mut router_with_middleware = router; // Create a new variable for clarity
    if app_state.config().rate_limit.enabled {
        info!("Rate limiting enabled. Applying tower-governor middleware...");

        let default_limit = &app_state.config().rate_limit.default_limit;
        let requests = default_limit.requests as u32;
        let window = default_limit.window;

        let requests_non_zero =
            NonZeroU32::new(requests).unwrap_or_else(|| {
                error!(
                    requests,
                    "Rate limit requests must be non-zero. Defaulting to 1."
                );
                NonZeroU32::new(1).unwrap()
            });

        if window == Duration::ZERO {
            let err_msg = "Rate limit window must be non-zero";
            error!(window = ?window, error = err_msg);
            return Err(Error::Config(ConfigError::Validation(format!(
                "Invalid rate limit configuration: {}",
                err_msg
            ))));
        }

        // Build the governor config
        let governor_config = GovernorConfigBuilder::default()
            .burst_size(requests_non_zero.get()) // Use .get() here
            .period(window) // Set replenishment period
            .key_extractor(PeerIpKeyExtractor) // Limit per IP
            .finish()
            .expect("Failed to create Governor config from valid parameters"); // Handle Option

        // Apply the GovernorLayer
        router_with_middleware = router_with_middleware.layer(GovernorLayer {
            config: Arc::new(governor_config),
        });

        info!("tower-governor middleware applied.");
    } else {
        info!(
            "Global rate limiting disabled. Skipping tower-governor middleware."
        );
    }

    if let Some(cors_layer) = app_state.config().build_cors_layer() {
        info!("Applying CORS middleware based on configuration...");
        router_with_middleware = router_with_middleware.layer(cors_layer); // Apply to router_with_middleware
        info!("CORS middleware applied.");
    } else {
        info!("CORS middleware disabled in configuration.");
    }

    // 4. Prepare for binding and serving
    let bind_address = app_state.config().http.bind_address;
    let axum_handle = Handle::new(); // Create the axum handle

    // Spawn the handler task BEFORE starting the server, passing only axum
    // handle
    spawn_graceful_shutdown_handler(axum_handle.clone());

    info!(%bind_address, tls_enabled = rustls_config.is_some(), "Attempting to bind server...");

    let serve_result = if let Some(tls) = rustls_config {
        info!("Binding HTTPS server to {}...", bind_address);
        axum_server::bind_rustls(bind_address, tls)
            .handle(axum_handle)
            .serve(router_with_middleware.into_make_service_with_connect_info::<SocketAddr>()) // Use router_with_middleware
            .await
            .map_err(|e| {
                error!(address = %bind_address, error = %e, "HTTPS server failed during operation");
                Error::Transport(format!("HTTPS Server error: {}", e))
            })
    } else {
        info!("Binding HTTP server to {}...", bind_address);
        let listener = TcpListener::bind(bind_address).await.map_err(|e| {
            error!(address = %bind_address, error = %e, "Failed to bind HTTP listener");
            Error::Transport(format!("Failed to bind to {}: {}", bind_address, e))
        })?;
        let actual_addr = listener.local_addr().map_err(|e| {
            error!(error = %e, "Failed to get local address after binding");
            Error::Internal(format!("Failed to get local address: {}", e))
        })?;
        info!(address = %actual_addr, "HTTP server listening");

        let std_listener = listener.into_std().map_err(|e| {
            error!(address = %actual_addr, error = %e, "Failed to convert listener to std");
            Error::Internal(format!("Listener conversion error: {}", e))
        })?;

        axum_server::from_tcp(std_listener)
            .handle(axum_handle)
            .serve(router_with_middleware.into_make_service_with_connect_info::<SocketAddr>()) // Use router_with_middleware
            .await
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
