// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod axum_app;
#[cfg(feature = "chain")]
mod chain;
#[cfg(feature = "chain")]
mod driver;
mod error;
mod event;
#[cfg(feature = "chain")]
mod graphql;
mod openapi;
mod policy;
#[cfg(feature = "prover")]
mod prover;
mod rues;
#[cfg(feature = "chain")]
mod rusk;
mod stream;

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "chain")]
use async_graphql::{BatchRequest, BatchResponse};
#[cfg(any(feature = "chain", feature = "prover", test))]
use async_trait::async_trait;
use axum::http::HeaderMap;
#[cfg(test)]
use axum::http::HeaderValue;
#[cfg(test)]
use axum::http::header::{ALLOW, CONTENT_TYPE};
#[cfg(feature = "chain")]
pub(crate) use driver::DriverExecutor;
pub use error::Error as HttpError;
pub(crate) use event::{
    DataType, ExecutionError, MessageResponse as EventResponse,
};
pub use policy::HttpPolicyConfig;
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinError;
use tokio::{io, task};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::info;

use self::axum_app::{HttpAppState, build_app};
pub use self::event::{RUES_LOCATION_PREFIX, RuesDispatchEvent, RuesEvent};
use self::event::{ResponseData, RuesEventUri, SessionId};
use self::policy::HttpRequestPolicy;
use self::stream::Listener;

pub type HttpResult<T> = std::result::Result<T, HttpError>;

const RUSK_VERSION_HEADER: &str = "Rusk-Version";
const RUSK_VERSION_STRICT_HEADER: &str = "Rusk-Version-Strict";
/// Default cap for most RUES POST request bodies.
#[cfg_attr(
    not(any(feature = "chain", feature = "prover", test)),
    allow(dead_code)
)]
pub(crate) const MAX_RUES_REQUEST_BODY_BYTES: usize = 3 * 1024 * 1024;
/// Cap for `POST /on/contract:<id>/upload_driver` request bodies.
#[cfg(feature = "chain")]
pub(crate) const MAX_DRIVER_UPLOAD_BODY_BYTES: usize = 2 * 1024 * 1024;
/// Cap for `POST /graphql` request bodies.
#[cfg(feature = "chain")]
pub(crate) const MAX_GRAPHQL_REQUEST_BODY_BYTES: usize = 256 * 1024;
/// Cap for a single inbound WebSocket message on the RUES subscription socket.
pub(crate) const MAX_WS_INBOUND_MESSAGE_BYTES: usize = 256 * 1024;
/// Cap for a single inbound WebSocket frame payload on the RUES subscription
/// socket.
pub(crate) const MAX_WS_INBOUND_FRAME_BYTES: usize = 64 * 1024;

pub struct HttpServer {
    handle: task::JoinHandle<()>,
    _shutdown: broadcast::Sender<Infallible>,
}

pub struct HttpServerConfig {
    pub address: String,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    pub enable_docs: bool,
    pub headers: HeaderMap,
    pub ws_event_channel_cap: usize,
    pub policy: HttpPolicyConfig,
}

impl HttpServer {
    pub async fn wait(self) -> Result<(), JoinError> {
        self.handle.await
    }

    pub async fn bind(
        services: HttpHandlers,
        event_sender: broadcast::Sender<RuesEvent>,
        config: HttpServerConfig,
    ) -> io::Result<(Self, SocketAddr)> {
        let cert_and_key = match (config.cert, config.key) {
            (Some(cert), Some(key)) => Some((cert, key)),
            _ => None,
        };
        let listener = match cert_and_key {
            Some(ck) => Listener::bind_tls(&config.address, ck).await,
            None => Listener::bind(&config.address).await,
        }?;

        let (shutdown_sender, mut shutdown_receiver) = broadcast::channel(1);
        let local_addr = listener.local_addr()?;

        info!("Starting HTTP Listener to {local_addr}");

        let mut headers = config.headers;
        headers.insert(
            RUSK_VERSION_HEADER,
            axum::http::HeaderValue::from_str(crate::VERSION.as_str())
                .expect("version should be a valid header value"),
        );

        let app = build_app(HttpAppState {
            services,
            sockets_map: Arc::new(RwLock::new(HashMap::new())),
            events: event_sender,
            shutdown: shutdown_sender.clone(),
            ws_event_channel_cap: config.ws_event_channel_cap,
            enable_docs: config.enable_docs,
            policy: Arc::new(HttpRequestPolicy::new(config.policy)),
            headers: Arc::new(headers),
        });

        // NormalizePathLayer strips trailing slashes so `/graphql/`
        // is served by the `/graphql` route without duplication.
        // It wraps the entire router so normalisation happens before
        // any routing or middleware runs.
        let app = NormalizePathLayer::trim_trailing_slash().layer(app);
        let make_svc =
            axum::ServiceExt::<axum::http::Request<axum::body::Body>>::into_make_service(app);

        let handle = task::spawn(async move {
            let shutdown_signal = async move {
                let _ = shutdown_receiver.recv().await;
            };
            let _ = axum::serve(listener, make_svc)
                .with_graceful_shutdown(shutdown_signal)
                .await;
        });

        let server = Self {
            handle,
            _shutdown: shutdown_sender,
        };
        Ok((server, local_addr))
    }
}

/// Registry of optional handler implementations.
#[derive(Default, Clone)]
pub struct HttpHandlers {
    #[cfg(feature = "chain")]
    /// Chain-owned RUES handlers for transaction, network, account, block,
    /// blob, and chain-state contract routes.
    chain: Option<Arc<dyn ChainRequestHandler>>,
    #[cfg(feature = "chain")]
    /// Rusk-owned RUES handlers for provisioner/CRS, contract query, driver,
    /// and contract metadata routes.
    rusk: Option<Arc<dyn RuskRequestHandler>>,
    #[cfg(feature = "chain")]
    /// Handler for the dedicated `/graphql` HTTP endpoint.
    graphql: Option<Arc<dyn GraphqlHandler>>,
    #[cfg(feature = "prover")]
    /// Handler for proof-generation routes under `/on/prover/*`.
    prover: Option<Arc<dyn ProverRequestHandler>>,
    #[cfg(test)]
    /// Test-only handler surface used by `/on/test/*` routes.
    test: Option<Arc<dyn TestRequestHandler>>,
}

#[cfg(feature = "chain")]
#[async_trait]
pub trait GraphqlHandler: Send + Sync + 'static {
    /// Execute a single or batch request received on the standalone `/graphql`
    /// route.
    async fn execute_graphql(&self, request: BatchRequest) -> BatchResponse;
}

#[cfg(feature = "chain")]
#[async_trait]
pub trait ChainRequestHandler: Send + Sync + 'static {
    /// Handle legacy GraphQL requests routed through `/on/graphql/query`.
    async fn graphql_query(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/transactions/{topic}` requests such as `preverify`,
    /// `propagate`, and `simulate`.
    async fn transactions(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/network/{topic}` requests for peer and network state.
    async fn network(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle chain node routes such as `/on/node/info`.
    /// This is node runtime/state information, not Rusk-owned provisioner or
    /// CRS data.
    async fn node(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/account:{entity}/{topic}` requests against chain state.
    async fn account(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle chain-owned `/on/contract:{entity}/{topic}` topics that expose
    /// chain status for a contract. Currently this is only the `status` topic
    /// (which is the contract balance).
    async fn contract(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/blocks/{topic}` requests.
    async fn blocks(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/blobs:{entity}/{topic}` requests.
    async fn blobs(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/stats/{topic}` requests for chain-derived statistics.
    async fn stats(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

#[cfg(feature = "chain")]
#[async_trait]
pub trait RuskRequestHandler: Send + Sync + 'static {
    /// Handle Rusk-owned `/on/node/{topic}` routes such as `provisioners` and
    /// `crs`.
    /// This is auxiliary node data exposed by Rusk, not general node info.
    async fn node(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/contracts:{entity}/{topic}` contract query and call routes.
    /// Unlike `contract`, this route dispatches contract-facing queries
    /// rather than single-contract metadata or driver operations.
    async fn contracts(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/driver:{entity}/{topic}` data-driver routes.
    async fn driver(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/contract_owner:{entity}/{topic}` owner lookup routes.
    async fn contract_owner(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle Rusk-owned `/on/contract:{entity}/{topic}` management and
    /// metadata topics such as `upload_driver`, `download_driver`, and
    /// `metadata`. This is operational contract management, not chain-state
    /// status.
    async fn contract(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

#[cfg(feature = "prover")]
#[async_trait]
pub trait ProverRequestHandler: Send + Sync + 'static {
    /// Handle `/on/prover/prove` proof-generation requests.
    async fn prove(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

#[cfg(test)]
#[async_trait]
pub trait TestRequestHandler: Send + Sync + 'static {
    /// Handle test-only `/on/test/{topic}` requests.
    async fn handle_test(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

impl HttpHandlers {
    #[cfg(feature = "chain")]
    pub fn set_chain_handler<T>(&mut self, handler: T)
    where
        T: ChainRequestHandler,
    {
        self.chain = Some(Arc::new(handler));
    }

    #[cfg(feature = "chain")]
    pub(crate) fn chain_handler(&self) -> Option<Arc<dyn ChainRequestHandler>> {
        self.chain.clone()
    }

    #[cfg(feature = "chain")]
    pub fn set_rusk_handler<T>(&mut self, handler: T)
    where
        T: RuskRequestHandler,
    {
        self.rusk = Some(Arc::new(handler));
    }

    #[cfg(feature = "chain")]
    pub(crate) fn rusk_handler(&self) -> Option<Arc<dyn RuskRequestHandler>> {
        self.rusk.clone()
    }

    #[cfg(feature = "chain")]
    pub fn set_graphql_handler<T>(&mut self, handler: T)
    where
        T: GraphqlHandler,
    {
        self.graphql = Some(Arc::new(handler));
    }

    #[cfg(feature = "chain")]
    pub(crate) fn graphql_handler(&self) -> Option<Arc<dyn GraphqlHandler>> {
        self.graphql.clone()
    }

    #[cfg(feature = "prover")]
    pub(crate) fn set_prover_handler<T>(&mut self, handler: T)
    where
        T: ProverRequestHandler,
    {
        self.prover = Some(Arc::new(handler));
    }

    #[cfg(feature = "prover")]
    pub(crate) fn prover_handler(
        &self,
    ) -> Option<Arc<dyn ProverRequestHandler>> {
        self.prover.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_test_handler<T>(&mut self, handler: T)
    where
        T: TestRequestHandler,
    {
        self.test = Some(Arc::new(handler));
    }

    #[cfg(test)]
    pub(crate) fn test_handler(&self) -> Option<Arc<dyn TestRequestHandler>> {
        self.test.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::net::{SocketAddr, TcpStream};
    use std::{fs, thread};

    #[cfg(feature = "chain")]
    use async_graphql::{
        BatchRequest, BatchResponse, Context, EmptyMutation, EmptySubscription,
        Object, Schema,
    };
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use dusk_core::abi::ContractId;
    use event::{BinaryWrapper, RequestData};
    use node_data::events::contract::{ContractEvent, ContractTxEvent};
    use tungstenite::{Message, client};

    use super::*;

    /// A [`TestRequestHandler`] implementation that returns the same data
    struct TestHandle;
    impl TestHandle {
        fn ok() -> HttpResult<ResponseData> {
            Ok(ResponseData::new("ok".to_string()))
        }
    }

    struct SlowRuesHandle {
        entered: std::sync::Arc<tokio::sync::Notify>,
        delay: std::time::Duration,
    }

    const STREAMED_DATA: &[&[u8; 16]] = &[
        b"I am call data 0",
        b"I am call data 1",
        b"I am call data 2",
        b"I am call data 3",
    ];

    #[async_trait]
    impl TestRequestHandler for TestHandle {
        async fn handle_test(
            &self,
            topic: &str,
            request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            let response = match topic {
                "stream" => {
                    let (sender, rec) = std::sync::mpsc::channel();
                    thread::spawn(move || {
                        for f in STREAMED_DATA.iter() {
                            sender.send(f.to_vec()).unwrap()
                        }
                    });
                    ResponseData::new(rec)
                }
                "echo" => ResponseData::new(request.data.as_bytes().to_vec()),
                "no-content" => ResponseData::new(DataType::None),
                "internal-error" => {
                    return Err(HttpError::internal("sensitive details"));
                }
                "invalid-header" => ResponseData::new("ok".to_string())
                    .with_header("bad\nheader", "value"),
                _ => return Err(HttpError::Unsupported),
            };
            Ok(response)
        }
    }

    #[async_trait]
    impl TestRequestHandler for SlowRuesHandle {
        async fn handle_test(
            &self,
            topic: &str,
            request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            match topic {
                "slow" => {
                    self.entered.notify_waiters();
                    tokio::time::sleep(self.delay).await;
                    Ok(ResponseData::new(request.data.as_bytes().to_vec()))
                }
                _ => Err(HttpError::Unsupported),
            }
        }
    }

    #[cfg(feature = "chain")]
    struct GraphqlQuery;

    #[cfg(feature = "chain")]
    #[Object]
    impl GraphqlQuery {
        async fn ping(&self) -> &'static str {
            "pong"
        }
    }

    #[cfg(feature = "chain")]
    struct TestGraphqlHandler;

    #[cfg(feature = "chain")]
    #[async_trait]
    impl GraphqlHandler for TestGraphqlHandler {
        async fn execute_graphql(
            &self,
            request: BatchRequest,
        ) -> BatchResponse {
            let schema =
                Schema::build(GraphqlQuery, EmptyMutation, EmptySubscription)
                    .finish();
            schema.execute_batch(request).await
        }
    }

    #[cfg(feature = "chain")]
    struct TestGraphqlHttpHeaderHandler;

    #[cfg(feature = "chain")]
    struct GraphqlHeaderQuery;

    #[cfg(feature = "chain")]
    #[Object]
    impl GraphqlHeaderQuery {
        async fn ping(&self, ctx: &Context<'_>) -> &'static str {
            let _ = ctx
                .insert_http_header("x-graphql-test-header", "set-by-handler");
            "pong"
        }
    }

    #[cfg(feature = "chain")]
    #[async_trait]
    impl GraphqlHandler for TestGraphqlHttpHeaderHandler {
        async fn execute_graphql(
            &self,
            request: BatchRequest,
        ) -> BatchResponse {
            let schema = Schema::build(
                GraphqlHeaderQuery,
                EmptyMutation,
                EmptySubscription,
            )
            .finish();
            schema.execute_batch(request).await
        }
    }

    // Keep these methods explicit so the tests still show which routes are
    // owned by each service trait.
    #[cfg(feature = "chain")]
    #[async_trait]
    impl ChainRequestHandler for TestHandle {
        async fn graphql_query(
            &self,
            request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            let gql_query = request.data.as_string();

            if gql_query.trim().is_empty() {
                return Ok(ResponseData::new("schema".to_string()));
            }

            if gql_query.contains("missing") {
                return Ok(ResponseData::new(serde_json::json!({
                    "data": null,
                    "errors": [{ "message": "missing field" }],
                })));
            }

            Ok(ResponseData::new(serde_json::json!({ "data": "ok" })))
        }

        async fn transactions(
            &self,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn network(
            &self,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn node(
            &self,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn account(
            &self,
            _entity: &str,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn contract(
            &self,
            _entity: &str,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn blocks(
            &self,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn blobs(
            &self,
            _entity: &str,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn stats(
            &self,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }
    }

    #[cfg(feature = "chain")]
    #[async_trait]
    impl RuskRequestHandler for TestHandle {
        async fn node(
            &self,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn contracts(
            &self,
            _entity: &str,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn driver(
            &self,
            _entity: &str,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn contract_owner(
            &self,
            _entity: &str,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }

        async fn contract(
            &self,
            _entity: &str,
            _topic: &str,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Self::ok()
        }
    }

    const EVENT_CHANNEL_CAP: usize = 16;
    const WS_EVENT_CHANNEL_CAP: usize = 2;

    #[derive(Default)]
    struct TestServerOptions {
        cert_and_key: Option<(&'static str, &'static str)>,
        policy: HttpPolicyConfig,
        headers: HeaderMap,
    }

    async fn bind_test_server_with_services(
        services: HttpHandlers,
        options: TestServerOptions,
    ) -> (HttpServer, SocketAddr, broadcast::Sender<RuesEvent>) {
        let (event_sender, _event_receiver) =
            broadcast::channel(EVENT_CHANNEL_CAP);
        let (_server, local_addr) = HttpServer::bind(
            services,
            event_sender.clone(),
            HttpServerConfig {
                address: "localhost:0".to_string(),
                cert: options.cert_and_key.map(|(c, _)| PathBuf::from(c)),
                key: options.cert_and_key.map(|(_, k)| PathBuf::from(k)),
                enable_docs: false,
                headers: options.headers,
                ws_event_channel_cap: WS_EVENT_CHANNEL_CAP,
                policy: options.policy,
            },
        )
        .await
        .expect("Binding the server to the address should succeed");

        (_server, local_addr, event_sender)
    }

    async fn bind_test_server<H: TestRequestHandler>(
        handler: H,
    ) -> (HttpServer, SocketAddr, broadcast::Sender<RuesEvent>) {
        bind_test_server_with_options(handler, TestServerOptions::default())
            .await
    }

    async fn bind_test_server_with_headers<H: TestRequestHandler>(
        handler: H,
        headers: HeaderMap,
    ) -> (HttpServer, SocketAddr, broadcast::Sender<RuesEvent>) {
        bind_test_server_with_options(
            handler,
            TestServerOptions {
                headers,
                ..Default::default()
            },
        )
        .await
    }

    async fn bind_test_server_with_options<H: TestRequestHandler>(
        handler: H,
        options: TestServerOptions,
    ) -> (HttpServer, SocketAddr, broadcast::Sender<RuesEvent>) {
        let mut services = HttpHandlers::default();
        services.set_test_handler(handler);
        bind_test_server_with_services(services, options).await
    }

    fn connect_ws(
        local_addr: SocketAddr,
    ) -> (tungstenite::WebSocket<TcpStream>, SessionId) {
        let stream = TcpStream::connect(local_addr)
            .expect("Connecting to the server should succeed");

        let ws_uri = format!("ws://{local_addr}/on");
        let (mut stream, _) = client(ws_uri, stream)
            .expect("Handshake with the server should succeed");

        let first_message =
            stream.read().expect("Session ID should be received");
        let sid = SessionId::parse(
            &first_message
                .into_text()
                .expect("Session ID should come in a text message"),
        )
        .expect("Session ID should be parsed");

        (stream, sid)
    }

    async fn assert_status_contains(
        response: reqwest::Response,
        status: StatusCode,
        expected: &str,
    ) {
        assert_eq!(response.status(), status);

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(content_type, "application/json");

        let body = response
            .text()
            .await
            .expect("Reading response body should succeed");
        let json: serde_json::Value = serde_json::from_str(&body)
            .expect("Error body should be valid JSON");
        let error_msg = json
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        assert!(
            error_msg.contains(expected),
            "Expected error containing '{expected}', got: {body}"
        );
    }

    async fn assert_bad_request_contains(
        response: reqwest::Response,
        expected: &str,
    ) {
        assert_status_contains(response, StatusCode::BAD_REQUEST, expected)
            .await;
    }

    async fn assert_graphql_error_contains(
        response: reqwest::Response,
        status: StatusCode,
        expected: &str,
    ) {
        assert_eq!(response.status(), status);

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(content_type, "application/json");

        let body = response
            .text()
            .await
            .expect("Reading response body should succeed");
        let json: serde_json::Value = serde_json::from_str(&body)
            .expect("Error body should be valid JSON");
        let error_msg = json
            .get("errors")
            .and_then(serde_json::Value::as_array)
            .and_then(|errors| errors.first())
            .and_then(|error| error.get("message"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        assert!(
            error_msg.contains(expected),
            "Expected GraphQL error containing '{expected}', got: {body}"
        );
    }

    #[cfg(feature = "chain")]
    async fn assert_graphql_ping_response(response: reqwest::Response) {
        assert_eq!(response.status(), StatusCode::OK);
        let payload_bytes =
            response.bytes().await.expect("Response should have body");
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .expect("Response should be JSON");
        assert_eq!(payload["data"]["ping"], "pong");
    }

    #[cfg(feature = "http-wasm")]
    async fn assert_wasm_response(response: reqwest::Response, path: &str) {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response_header(&response, "Content-Type"),
            "application/wasm",
        );
        assert_eq!(
            response_header(&response, "Cache-Control"),
            "public, max-age=31536000, immutable",
        );
        let body = response
            .bytes()
            .await
            .expect("Reading response body should succeed");
        assert!(
            !body.is_empty(),
            "WASM response body should not be empty for {path}"
        );
    }

    fn response_header(response: &reqwest::Response, header: &str) -> String {
        response
            .headers()
            .get(header)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string()
    }

    fn assert_retry_after_positive_integer(response: &reqwest::Response) {
        let retry_after = response
            .headers()
            .get("Retry-After")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert!(
            !retry_after.is_empty(),
            "429 responses should carry Retry-After",
        );
        let retry_after = retry_after
            .parse::<u64>()
            .expect("Retry-After should be a positive integer");
        assert!(retry_after >= 1, "Retry-After should be at least 1 second");
    }

    #[tokio::test]
    async fn http_query() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let data = Vec::from(&b"I am call data 0"[..]);
        let data = RequestData::Binary(BinaryWrapper { inner: data });

        let request_bytes = data.as_bytes();

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/on/test/echo", local_addr))
            .body(request_bytes.to_vec())
            .send()
            .await
            .expect("Requesting should succeed");

        let response_bytes =
            response.bytes().await.expect("There should be a response");
        let response_bytes =
            hex::decode(response_bytes).expect("data to be hex encoded");

        assert_eq!(
            request_bytes, response_bytes,
            "Data received the same as sent"
        );
    }

    #[tokio::test]
    async fn unsupported_http_path_returns_not_found() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{local_addr}/unsupported"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::NOT_FOUND,
            "Path not found",
        )
        .await;
    }

    #[tokio::test]
    async fn post_rues_empty_response_returns_accepted_with_empty_body() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/no-content"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = response
            .bytes()
            .await
            .expect("Reading response body should succeed");
        assert!(body.is_empty(), "Expected empty response body");
    }

    #[tokio::test]
    async fn post_rues_oversized_body_returns_payload_too_large() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let oversized = vec![b'a'; MAX_RUES_REQUEST_BODY_BYTES + 1];

        let response = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .body(oversized)
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn policy_acl_denied_request_returns_forbidden() {
        let mut policy = HttpPolicyConfig::default();
        policy.acl.rules.push(policy::HttpPolicyAclRule {
            id: "deny-test-echo".to_string(),
            enabled: true,
            action: policy::HttpPolicyAclAction::Deny,
            path: "/on/test/echo".to_string(),
            method: vec!["POST".to_string()],
            headers: HashMap::new(),
        });

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_options(
                TestHandle,
                TestServerOptions {
                    policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response
            .text()
            .await
            .expect("Reading response body should succeed");
        assert!(
            body.contains("forbidden"),
            "Forbidden response body should contain error marker"
        );
    }

    #[tokio::test]
    async fn policy_graphql_class_rate_limit_rejects_with_too_many_requests() {
        let mut policy = HttpPolicyConfig::default();
        policy.global_limits.classes.graphql = policy::HttpPolicyClassLimit {
            rps: 1,
            burst: 1,
            concurrency: 64,
        };

        let mut services = HttpHandlers::default();
        services.set_graphql_handler(TestGraphqlHandler);
        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions {
                    policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();
        let first = client
            .post(format!("http://{local_addr}/graphql"))
            .header("Content-Type", "application/json")
            .body(serde_json::json!({ "query": "{ ping }" }).to_string())
            .send()
            .await
            .expect("First request should complete");
        assert_eq!(first.status(), StatusCode::OK);

        let second = client
            .post(format!("http://{local_addr}/graphql"))
            .header("Content-Type", "application/json")
            .body(serde_json::json!({ "query": "{ ping }" }).to_string())
            .send()
            .await
            .expect("Second request should complete");
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_retry_after_positive_integer(&second);
    }

    #[cfg(feature = "chain")]
    #[tokio::test]
    async fn policy_tx_propagate_class_rate_limit_rejects_with_too_many_requests()
     {
        let mut policy = HttpPolicyConfig::default();
        policy.global_limits.classes.tx_propagate =
            policy::HttpPolicyClassLimit {
                rps: 1,
                burst: 1,
                concurrency: 64,
            };

        let mut services = HttpHandlers::default();
        services.set_chain_handler(TestHandle);
        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions {
                    policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();
        let first = client
            .post(format!("http://{local_addr}/on/transactions/propagate"))
            .body("tx")
            .send()
            .await
            .expect("First request should complete");
        assert_eq!(first.status(), StatusCode::OK);

        let second = client
            .post(format!("http://{local_addr}/on/transactions/propagate"))
            .body("tx")
            .send()
            .await
            .expect("Second request should complete");
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_retry_after_positive_integer(&second);
    }

    #[tokio::test]
    async fn policy_non_rues_paths_use_other_http_limits() {
        let mut policy = HttpPolicyConfig::default();
        policy.global_limits.classes.other_http =
            policy::HttpPolicyClassLimit {
                rps: 1,
                burst: 1,
                concurrency: 64,
            };

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_options(
                TestHandle,
                TestServerOptions {
                    policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();
        let first = client
            .get(format!("http://{local_addr}/unknown"))
            .send()
            .await
            .expect("First request should complete");
        assert_eq!(first.status(), StatusCode::NOT_FOUND);

        let second = client
            .get(format!("http://{local_addr}/unknown"))
            .send()
            .await
            .expect("Second request should complete");
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_retry_after_positive_integer(&second);
    }

    #[tokio::test]
    async fn configured_headers_are_added_to_policy_rejections() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test-header", HeaderValue::from_static("test-value"));

        let mut forbidden_policy = HttpPolicyConfig::default();
        forbidden_policy.acl.rules.push(policy::HttpPolicyAclRule {
            id: "deny-test-echo".to_string(),
            enabled: true,
            action: policy::HttpPolicyAclAction::Deny,
            path: "/on/test/echo".to_string(),
            method: vec!["POST".to_string()],
            headers: HashMap::new(),
        });

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_options(
                TestHandle,
                TestServerOptions {
                    headers: headers.clone(),
                    policy: forbidden_policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();
        let forbidden = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
        assert_eq!(response_header(&forbidden, "x-test-header"), "test-value");

        let mut rate_limited_policy = HttpPolicyConfig::default();
        rate_limited_policy.global_limits.classes.other_http =
            policy::HttpPolicyClassLimit {
                rps: 1,
                burst: 1,
                concurrency: 64,
            };

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_options(
                TestHandle,
                TestServerOptions {
                    headers,
                    policy: rate_limited_policy,
                    ..Default::default()
                },
            )
            .await;

        let first = client
            .get(format!("http://{local_addr}/unknown"))
            .send()
            .await
            .expect("First request should complete");
        assert_eq!(first.status(), StatusCode::NOT_FOUND);

        let second = client
            .get(format!("http://{local_addr}/unknown"))
            .send()
            .await
            .expect("Second request should complete");
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response_header(&second, "x-test-header"), "test-value");
        assert_retry_after_positive_integer(&second);
    }

    #[tokio::test]
    async fn policy_other_rues_concurrency_limit_rejects_with_too_many_requests()
     {
        let mut policy = HttpPolicyConfig::default();
        policy.global_limits.classes.other_rues =
            policy::HttpPolicyClassLimit {
                rps: 2,
                burst: 2,
                concurrency: 1,
            };

        let entered = std::sync::Arc::new(tokio::sync::Notify::new());
        let (_server, local_addr, _event_sender) =
            bind_test_server_with_options(
                SlowRuesHandle {
                    entered: entered.clone(),
                    delay: std::time::Duration::from_millis(250),
                },
                TestServerOptions {
                    policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();

        let first_client = client.clone();
        let first_request = tokio::spawn(async move {
            first_client
                .post(format!("http://{local_addr}/on/test/slow"))
                .body("first")
                .send()
                .await
                .expect("First request should complete")
        });

        entered.notified().await;

        let second = client
            .post(format!("http://{local_addr}/on/test/slow"))
            .body("second")
            .send()
            .await
            .expect("Second request should complete");
        let first = first_request
            .await
            .expect("First request task should complete");

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_retry_after_positive_integer(&second);
    }

    #[cfg(feature = "chain")]
    #[tokio::test]
    async fn policy_contract_and_feeder_query_classes_use_distinct_limits() {
        let mut policy = HttpPolicyConfig::default();
        policy.global_limits.classes.contract_query =
            policy::HttpPolicyClassLimit {
                rps: 1,
                burst: 1,
                concurrency: 64,
            };
        policy.global_limits.classes.feeder_query =
            policy::HttpPolicyClassLimit {
                rps: 1,
                burst: 1,
                concurrency: 64,
            };

        let mut services = HttpHandlers::default();
        services.set_rusk_handler(TestHandle);
        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions {
                    policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();
        let path = "http://{local_addr}/on/contracts:abcd/query";

        let first_contract = client
            .post(path.replace("{local_addr}", &local_addr.to_string()))
            .body("query")
            .send()
            .await
            .expect("First contract query should complete");
        assert_eq!(first_contract.status(), StatusCode::OK);

        let second_contract = client
            .post(path.replace("{local_addr}", &local_addr.to_string()))
            .body("query")
            .send()
            .await
            .expect("Second contract query should complete");
        assert_eq!(second_contract.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_retry_after_positive_integer(&second_contract);

        let first_feeder = client
            .post(path.replace("{local_addr}", &local_addr.to_string()))
            .header("Rusk-Feeder", "1")
            .body("query")
            .send()
            .await
            .expect("First feeder query should complete");
        assert_eq!(first_feeder.status(), StatusCode::OK);

        let second_feeder = client
            .post(path.replace("{local_addr}", &local_addr.to_string()))
            .header("Rusk-Feeder", "1")
            .body("query")
            .send()
            .await
            .expect("Second feeder query should complete");
        assert_eq!(second_feeder.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_retry_after_positive_integer(&second_feeder);
    }

    #[cfg(feature = "chain")]
    #[tokio::test]
    async fn policy_upload_driver_class_rate_limit_rejects_with_too_many_requests()
     {
        let mut policy = HttpPolicyConfig::default();
        policy.global_limits.classes.upload_driver =
            policy::HttpPolicyClassLimit {
                rps: 1,
                burst: 1,
                concurrency: 64,
            };

        let mut services = HttpHandlers::default();
        services.set_rusk_handler(TestHandle);
        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions {
                    policy,
                    ..Default::default()
                },
            )
            .await;

        let client = reqwest::Client::new();
        let path =
            format!("http://{local_addr}/on/contract:abcd/upload_driver");

        let first = client
            .post(&path)
            .body("driver")
            .send()
            .await
            .expect("First upload request should complete");
        assert_eq!(first.status(), StatusCode::OK);

        let second = client
            .post(&path)
            .body("driver")
            .send()
            .await
            .expect("Second upload request should complete");
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_retry_after_positive_integer(&second);
    }

    #[tokio::test]
    async fn post_rues_upload_driver_oversized_body_returns_payload_too_large()
    {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        const CONTRACT_ID: ContractId = ContractId::from_bytes([1; 32]);
        let contract_id_hex = hex::encode(CONTRACT_ID);

        let client = reqwest::Client::new();
        let oversized = vec![0u8; MAX_DRIVER_UPLOAD_BODY_BYTES + 1];
        let response = client
            .post(format!(
                "http://{local_addr}/on/contract:{contract_id_hex}/upload_driver"
            ))
            .body(oversized)
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn post_rues_strict_without_version_returns_bad_request() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .header(RUSK_VERSION_STRICT_HEADER, "1")
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_bad_request_contains(
            response,
            "Missing Rusk-Version header while Rusk-Version-Strict is set",
        )
        .await;
    }

    #[tokio::test]
    async fn post_rues_invalid_version_header_encoding_returns_bad_request() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let invalid_version = HeaderValue::from_bytes(&[0xff])
            .expect("Creating invalid UTF-8 header value should succeed");

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .header(RUSK_VERSION_HEADER, invalid_version)
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_bad_request_contains(
            response,
            "Invalid Rusk-Version header encoding",
        )
        .await;
    }

    #[tokio::test]
    async fn post_rues_on_without_path_rejected() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        // POST to /on (WS-only endpoint) is rejected by axum because
        // the WebSocketUpgrade extractor requires an upgrade request.
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on"))
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn post_rues_invalid_utf8_payload_returns_unprocessable_entity() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .body(vec![0xff, 0xfe])
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid utf8",
        )
        .await;
    }

    #[tokio::test]
    async fn request_parse_other_http_internal_is_sanitized() {
        let response =
            error::ApiError::from(HttpError::internal("sensitive details"))
                .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let body = String::from_utf8(body.to_vec())
            .expect("response body should be utf-8 json");

        assert!(body.contains("Internal server error"));
        assert!(!body.contains("sensitive details"));
    }

    #[tokio::test]
    async fn post_rues_unsupported_route_returns_json_error() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/unsupported"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::NOT_IMPLEMENTED,
            "Unsupported operation",
        )
        .await;
    }

    #[tokio::test]
    async fn post_rues_internal_handler_error_is_sanitized() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/internal-error"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error",
        )
        .await;
    }

    #[tokio::test]
    async fn post_rues_invalid_response_header_returns_unprocessable_entity() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/invalid-header"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid header",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_delete_rues_strict_without_version_returns_bad_request() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;
        let (_stream, sid) = connect_ws(local_addr);

        let client = reqwest::Client::new();
        let contract_id_hex = hex::encode(ContractId::from_bytes([1; 32]));
        let path =
            format!("http://{local_addr}/on/contracts:{contract_id_hex}/topic");

        let subscribe_response = client
            .get(path.clone())
            .header("Rusk-Session-Id", sid.to_string())
            .header(RUSK_VERSION_STRICT_HEADER, "1")
            .send()
            .await
            .expect("Requesting should succeed");
        assert_bad_request_contains(
            subscribe_response,
            "Missing Rusk-Version header while Rusk-Version-Strict is set",
        )
        .await;

        let unsubscribe_response = client
            .delete(path)
            .header("Rusk-Session-Id", sid.to_string())
            .header(RUSK_VERSION_STRICT_HEADER, "1")
            .send()
            .await
            .expect("Requesting should succeed");
        assert_bad_request_contains(
            unsubscribe_response,
            "Missing Rusk-Version header while Rusk-Version-Strict is set",
        )
        .await;
    }

    #[tokio::test]
    async fn rues_session_and_path_validation_matrix() {
        struct Case {
            method: reqwest::Method,
            path: String,
            session_id: Option<&'static str>,
            expected_status: StatusCode,
            expected_error: &'static str,
        }

        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;
        let contract_id_hex = hex::encode(ContractId::from_bytes([8; 32]));
        let topic_path = format!("/on/contracts:{contract_id_hex}/topic");
        let client = reqwest::Client::new();

        let cases = vec![
            Case {
                method: reqwest::Method::GET,
                path: topic_path.clone(),
                session_id: None,
                expected_status: StatusCode::FAILED_DEPENDENCY,
                expected_error: "Session ID not provided or invalid",
            },
            // Non-WS requests to /on (without subpath) are rejected by
            // axum's WebSocketUpgrade extractor (400). Only /on/{*path}
            // cases are validated here.
            Case {
                method: reqwest::Method::GET,
                path: topic_path.clone(),
                session_id: Some("invalid-session-id"),
                expected_status: StatusCode::FAILED_DEPENDENCY,
                expected_error: "Session ID not provided or invalid",
            },
            Case {
                method: reqwest::Method::DELETE,
                path: topic_path,
                session_id: Some("invalid-session-id"),
                expected_status: StatusCode::FAILED_DEPENDENCY,
                expected_error: "Session ID not provided or invalid",
            },
            // Non-WS requests to /on are rejected by axum's
            // WebSocketUpgrade extractor (400 Bad Request). Tests for
            // /on/{*path} method routing live in
            // put_rues_returns_method_not_allowed.
        ];

        for case in cases {
            let mut req = client.request(
                case.method.clone(),
                format!("http://{local_addr}{}", case.path),
            );
            if let Some(session_id) = case.session_id {
                req = req.header("Rusk-Session-Id", session_id);
            }
            let response = req.send().await.expect("Requesting should succeed");
            assert_status_contains(
                response,
                case.expected_status,
                case.expected_error,
            )
            .await;
        }
    }

    #[tokio::test]
    async fn put_rues_returns_method_not_allowed() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        // Axum's method router rejects PUT at the routing layer — no WS
        // session needed.
        let client = reqwest::Client::new();
        let contract_id_hex = hex::encode(ContractId::from_bytes([7; 32]));
        let path =
            format!("http://{local_addr}/on/contracts:{contract_id_hex}/topic");
        let response = client
            .put(path)
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let allow_header = response
            .headers()
            .get(ALLOW)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();

        // Axum lists all registered methods for the route.
        assert!(
            allow_header.contains("DELETE"),
            "Allow header should include DELETE: {allow_header}"
        );
        assert!(
            allow_header.contains("GET"),
            "Allow header should include GET: {allow_header}"
        );
        assert!(
            allow_header.contains("POST"),
            "Allow header should include POST: {allow_header}"
        );
    }

    #[tokio::test]
    async fn https_query() {
        let provider =
            tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
        let _ = provider.install_default();

        let cert_path = "tests/assets/cert.pem";
        let key_path = "tests/assets/key.pem";

        let cert_bytes = fs::read(cert_path).expect("cert file should exist");
        let certificate = reqwest::tls::Certificate::from_pem(&cert_bytes)
            .expect("cert should be valid");

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_options(
                TestHandle,
                TestServerOptions {
                    cert_and_key: Some((cert_path, key_path)),
                    ..Default::default()
                },
            )
            .await;

        let data = Vec::from(&b"I am call data 0"[..]);
        let data = RequestData::Binary(BinaryWrapper { inner: data });
        let request_bytes = data.as_bytes().to_vec();

        let client = reqwest::ClientBuilder::new()
            .add_root_certificate(certificate)
            .danger_accept_invalid_certs(true)
            .build()
            .expect("creating client should succeed");

        let response = client
            .post(format!(
                "https://localhost:{}/on/test/echo",
                local_addr.port()
            ))
            .body(request_bytes.clone())
            .send()
            .await
            .expect("Requesting should succeed");

        let response_bytes =
            response.bytes().await.expect("There should be a response");
        let response_bytes =
            hex::decode(response_bytes).expect("data to be hex encoded");

        assert_eq!(
            request_bytes, response_bytes,
            "Data received the same as sent"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues() {
        let (_server, local_addr, event_sender) =
            bind_test_server(TestHandle).await;
        let (mut stream, sid) = connect_ws(local_addr);

        const SUB_CONTRACT_ID: ContractId = ContractId::from_bytes([1; 32]);
        const MAYBE_SUB_CONTRACT_ID: ContractId =
            ContractId::from_bytes([2; 32]);
        const NON_SUB_CONTRACT_ID: ContractId = ContractId::from_bytes([3; 32]);

        const TOPIC: &str = "topic";

        let sub_contract_id_hex = hex::encode(SUB_CONTRACT_ID);
        let maybe_sub_contract_id_hex = hex::encode(MAYBE_SUB_CONTRACT_ID);

        let client = reqwest::Client::new();

        let response = client
            .get(format!(
                "http://{local_addr}/on/contracts:{sub_contract_id_hex}/{TOPIC}",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .get(format!(
                "http://{local_addr}/on/contracts:{maybe_sub_contract_id_hex}/{TOPIC}",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        // This event is subscribed to, so it should be received
        let received_event = RuesEvent::from(ContractTxEvent {
            event: ContractEvent {
                target: SUB_CONTRACT_ID,
                topic: TOPIC.into(),
                data: b"hello, events".to_vec(),
            },
            origin: [0; 32],
        });

        // This event is at first subscribed to, so it should be received the
        // first time
        let at_first_received_event = RuesEvent::from(ContractTxEvent {
            event: ContractEvent {
                target: MAYBE_SUB_CONTRACT_ID,
                topic: TOPIC.into(),
                data: b"hello, events".to_vec(),
            },
            origin: [1; 32],
        });

        // This event is not subscribed to, so it should not be received
        let non_received_event = RuesEvent::from(ContractTxEvent {
            event: ContractEvent {
                target: NON_SUB_CONTRACT_ID,
                topic: TOPIC.into(),
                data: b"hello, events".to_vec(),
            },
            origin: [2; 32],
        });

        event_sender
            .send(non_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(at_first_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(received_event.clone())
            .expect("Sending event should succeed");

        let message = stream.read().expect("Event should be received");
        let event_bytes = message.into_data();

        let event = from_bytes(&event_bytes).expect("Event should deserialize");

        assert_eq!(at_first_received_event, event, "Event should be the same");

        let message = stream.read().expect("Event should be received");
        let event_bytes = message.into_data();

        let event = from_bytes(&event_bytes).expect("Event should deserialize");

        assert_eq!(received_event, event, "Event should be the same");

        let response = client
            .delete(format!(
                "http://{local_addr}/on/contracts:{maybe_sub_contract_id_hex}/{TOPIC}",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        event_sender
            .send(non_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(at_first_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(received_event.clone())
            .expect("Sending event should succeed");

        let message = stream.read().expect("Event should be received");

        let event_bytes = message.into_data();

        let event = from_bytes(&event_bytes).expect("Event should deserialize");

        assert_eq!(received_event, event, "Event should be the same");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues_oversized_message_closes_connection() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;
        let (mut stream, _sid) = connect_ws(local_addr);

        stream
            .get_mut()
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .expect("setting TCP read timeout should succeed");

        let oversized = vec![0u8; MAX_WS_INBOUND_FRAME_BYTES + 1];
        let _ = stream.send(Message::Binary(oversized));

        match stream.read() {
            Ok(Message::Close(_)) => {}
            Ok(msg) => {
                panic!("Expected close after oversized message, got {msg:?}")
            }
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                ) =>
            {
                panic!("Timed out waiting for close after oversized message");
            }
            Err(_) => {}
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues_handshake_returns_switching_protocols_and_session_id()
     {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let stream = TcpStream::connect(local_addr)
            .expect("Connecting to the server should succeed");

        let ws_uri = format!("ws://{local_addr}/on");
        let (mut stream, response) = client(ws_uri, stream)
            .expect("Handshake with the server should succeed");

        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
        let upgrade = response
            .headers()
            .get("Upgrade")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        assert_eq!(upgrade, "websocket");
        assert!(
            response.headers().contains_key("Sec-WebSocket-Accept"),
            "Handshake should contain Sec-WebSocket-Accept"
        );

        let first_message =
            stream.read().expect("Session ID should be received");
        let sid_text = first_message
            .into_text()
            .expect("Session ID should come in a text message");
        assert_eq!(
            sid_text.len(),
            32,
            "Session ID should be a 16-byte hex string"
        );
        assert!(
            SessionId::parse(&sid_text).is_some(),
            "Session ID should be parseable"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues_missing_topic() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;
        let (_stream, sid) = connect_ws(local_addr);

        const CONTRACT_ID: ContractId = ContractId::from_bytes([1; 32]);
        let contract_id_hex = hex::encode(CONTRACT_ID);

        let client = reqwest::Client::new();

        // Subscribing without a topic (trailing slash) should fail
        let response = client
            .get(format!(
                "http://{local_addr}/on/contracts:{contract_id_hex}/",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Missing topic should return NOT_FOUND"
        );

        // Subscribing without a topic (no trailing slash) should also fail
        let response = client
            .get(format!(
                "http://{local_addr}/on/contracts:{contract_id_hex}",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Missing topic should return NOT_FOUND"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues_missing_contract_entity() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;
        let (_stream, sid) = connect_ws(local_addr);

        const TOPIC: &str = "withdraw";

        let client = reqwest::Client::new();

        // Subscribing to contracts without entity should fail
        let response = client
            .get(format!("http://{local_addr}/on/contracts/{TOPIC}"))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Contracts without entity should return NOT_FOUND"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues_unsubscribe_not_found() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;
        let (_stream, sid) = connect_ws(local_addr);

        let contract_id_hex = hex::encode(ContractId::from_bytes([9; 32]));
        let client = reqwest::Client::new();

        let response = client
            .delete(format!(
                "http://{local_addr}/on/contracts:{contract_id_hex}/topic",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response
            .text()
            .await
            .expect("Reading response body should succeed");
        assert!(
            body.contains("Subscription not found"),
            "Expected missing subscription error, got: {body}"
        );
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn legacy_graphql_query_still_works() {
        let mut services = HttpHandlers::default();
        services.set_chain_handler(TestHandle);

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions::default(),
            )
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/graphql/query"))
            .body("{ ping }")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload_bytes =
            response.bytes().await.expect("Response should have body");
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .expect("Response should be JSON");
        assert_eq!(payload["data"], "ok");
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn legacy_graphql_empty_body_returns_schema() {
        let mut services = HttpHandlers::default();
        services.set_chain_handler(TestHandle);

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions::default(),
            )
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/graphql/query"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.text().await.expect("Response should have body");
        assert_eq!(body, "schema");
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn legacy_graphql_invalid_query_returns_errors() {
        let mut services = HttpHandlers::default();
        services.set_chain_handler(TestHandle);

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions::default(),
            )
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/graphql/query"))
            .body("{ missing }")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload_bytes =
            response.bytes().await.expect("Response should have body");
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .expect("Response should be JSON");
        assert!(payload["errors"].is_array());
        assert_eq!(payload["errors"][0]["message"], "missing field");
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_post_query() {
        let mut services = HttpHandlers::default();
        services.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions::default(),
            )
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/graphql"))
            .header("Content-Type", "application/json")
            .body(serde_json::json!({ "query": "{ ping }" }).to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload_bytes =
            response.bytes().await.expect("Response should have body");
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .expect("Response should be JSON");
        assert_eq!(payload["data"]["ping"], "pong");
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_post_oversized_body_returns_payload_too_large() {
        let mut services = HttpHandlers::default();
        services.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions::default(),
            )
            .await;

        let client = reqwest::Client::new();
        let oversized = vec![b'a'; MAX_GRAPHQL_REQUEST_BODY_BYTES + 1];
        let response = client
            .post(format!("http://{local_addr}/graphql"))
            .header("Content-Type", "application/json")
            .body(oversized)
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_post_invalid_query_returns_errors() {
        let mut services = HttpHandlers::default();
        services.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions::default(),
            )
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/graphql"))
            .header("Content-Type", "application/json")
            .body(serde_json::json!({ "query": "{ missing }" }).to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload_bytes =
            response.bytes().await.expect("Response should have body");
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .expect("Response should be JSON");
        assert!(payload["errors"].is_array());
        assert!(!payload["errors"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_response_headers_from_handler_are_preserved() {
        let mut services = HttpHandlers::default();
        services.set_graphql_handler(TestGraphqlHttpHeaderHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_services(
                services,
                TestServerOptions::default(),
            )
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/graphql"))
            .header("Content-Type", "application/json")
            .body(serde_json::json!({ "query": "{ ping }" }).to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let header = response
            .headers()
            .get("x-graphql-test-header")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(header, "set-by-handler");
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_route_behavior_matrix() {
        struct Case {
            method: reqwest::Method,
            path: &'static str,
            configure_handler: bool,
            request_body: Option<&'static str>,
            expected_status: StatusCode,
            expected_error: Option<&'static str>,
            expected_allow: Option<&'static str>,
            expect_ping: bool,
        }

        let cases = [
            Case {
                method: reqwest::Method::GET,
                path: "/graphql?query=%7Bping%7D",
                configure_handler: true,
                request_body: None,
                expected_status: StatusCode::OK,
                expected_error: None,
                expected_allow: None,
                expect_ping: true,
            },
            Case {
                method: reqwest::Method::POST,
                path: "/graphql/",
                configure_handler: true,
                request_body: Some(r#"{"query":"{ ping }"}"#),
                expected_status: StatusCode::OK,
                expected_error: None,
                expected_allow: None,
                expect_ping: true,
            },
            Case {
                method: reqwest::Method::GET,
                path: "/graphql",
                configure_handler: true,
                request_body: None,
                expected_status: StatusCode::BAD_REQUEST,
                expected_error: Some("require a query parameter"),
                expected_allow: None,
                expect_ping: false,
            },
            Case {
                method: reqwest::Method::PUT,
                path: "/graphql",
                configure_handler: true,
                request_body: None,
                expected_status: StatusCode::METHOD_NOT_ALLOWED,
                expected_error: None,
                expected_allow: None,
                expect_ping: false,
            },
            Case {
                method: reqwest::Method::GET,
                path: "/graphql?query=%7Bping%7D",
                configure_handler: false,
                request_body: None,
                expected_status: StatusCode::NOT_FOUND,
                expected_error: Some("GraphQL endpoint not configured"),
                expected_allow: None,
                expect_ping: false,
            },
        ];

        let client = reqwest::Client::new();
        for case in cases {
            let mut services = HttpHandlers::default();
            if case.configure_handler {
                services.set_graphql_handler(TestGraphqlHandler);
            }
            let (_server, local_addr, _event_sender) =
                bind_test_server_with_services(
                    services,
                    TestServerOptions::default(),
                )
                .await;

            let mut request = client.request(
                case.method.clone(),
                format!("http://{local_addr}{}", case.path),
            );
            if let Some(body) = case.request_body {
                request = request
                    .header("Content-Type", "application/json")
                    .body(body.to_string());
            }
            let response =
                request.send().await.expect("Requesting should succeed");
            let allow_header = response_header(&response, ALLOW.as_str());

            if case.expect_ping {
                assert_graphql_ping_response(response).await;
            } else if let Some(expected_error) = case.expected_error {
                assert_graphql_error_contains(
                    response,
                    case.expected_status,
                    expected_error,
                )
                .await;
            } else {
                assert_eq!(response.status(), case.expected_status);
            }

            if let Some(expected_allow) = case.expected_allow {
                assert_eq!(allow_header, expected_allow);
            }
        }
    }

    #[tokio::test]
    #[cfg(feature = "http-wasm")]
    async fn http_wasm_paths_return_wasm_with_cache_headers() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let paths = [
            "/static/drivers/wallet-core.wasm",
            "/static/drivers/wallet-core-1.0.1.wasm",
            "/static/drivers/wallet-core-1.3.0.wasm",
            "/static/drivers/wallet-core-1.6.0.wasm",
        ];

        let client = reqwest::Client::new();
        for path in paths {
            let response = client
                .get(format!("http://{local_addr}{path}"))
                .send()
                .await
                .expect("Requesting should succeed");
            assert_wasm_response(response, path).await;
        }
    }

    #[tokio::test]
    async fn configured_headers_are_added_to_success_and_error_responses() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test-header", HeaderValue::from_static("test-value"));

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_headers(TestHandle, headers).await;

        let client = reqwest::Client::new();
        let success = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");
        assert_eq!(success.status(), StatusCode::OK);
        assert_eq!(response_header(&success, "x-test-header"), "test-value");

        let error = client
            .post(format!("http://{local_addr}/on/test/unsupported"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response_header(&error, "x-test-header"), "test-value");

        assert_status_contains(
            error,
            StatusCode::NOT_IMPLEMENTED,
            "Unsupported operation",
        )
        .await;
    }

    #[tokio::test]
    async fn post_rues_x_headers_are_reflected_case_insensitively() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .header("X-Trace-Id", "trace-123")
            .header("Authorization", "secret")
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let trace_header = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(trace_header, "trace-123");
        assert!(
            response.headers().get("authorization").is_none(),
            "Non x-* headers should not be reflected"
        );
    }

    fn parse_len(bytes: &[u8]) -> anyhow::Result<(usize, &[u8])> {
        if bytes.len() < 4 {
            return Err(anyhow::anyhow!("not enough bytes"));
        }

        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            as usize;
        let (_, left) = bytes.split_at(4);

        Ok((len, left))
    }

    type Header<'a> = (serde_json::Map<String, serde_json::Value>, &'a [u8]);
    pub(crate) fn parse_header<'a>(
        bytes: &'a [u8],
    ) -> anyhow::Result<Header<'a>> {
        let (len, bytes) = parse_len(bytes)?;
        if bytes.len() < len {
            return Err(anyhow::anyhow!(
                "not enough bytes for parsed len {len}"
            ));
        }

        let (header_bytes, bytes) = bytes.split_at(len);
        let header = serde_json::from_slice(header_bytes)?;

        Ok((header, bytes))
    }

    pub fn from_bytes(data: &[u8]) -> anyhow::Result<RuesEvent> {
        let (mut headers, data) = parse_header(data)?;

        let path = headers
            .remove("Content-Location")
            .ok_or(anyhow::anyhow!("Content location is not set"))?
            .as_str()
            .ok_or(anyhow::anyhow!("Content location is not a string"))?
            .to_string();

        let path = path
            .strip_prefix(RUES_LOCATION_PREFIX)
            .ok_or(anyhow::anyhow!("Invalid location prefix"))?;
        let mut segments = path.split('/');
        let _leading = segments.next();
        let target = segments
            .next()
            .ok_or(anyhow::anyhow!("Missing target in location"))?;
        let topic = segments
            .next()
            .filter(|segment| !segment.is_empty())
            .ok_or(anyhow::anyhow!("Missing topic in location"))?;
        if segments.next().is_some() {
            return Err(anyhow::anyhow!("Invalid location"));
        }
        let uri = RuesEventUri::from_target_and_topic(target, topic)
            .ok_or(anyhow::anyhow!("Invalid location"))?;

        let data = data.to_vec().into();
        Ok(RuesEvent { data, headers, uri })
    }
}
