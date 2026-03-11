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
mod policy;
#[cfg(feature = "prover")]
mod prover;
mod responses;
mod rues;
#[cfg(feature = "chain")]
mod rusk;
mod stream;

#[cfg(feature = "chain")]
pub(crate) use driver::DriverExecutor;
pub(crate) use event::{
    DataType, ExecutionError, MessageResponse as EventResponse,
};

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "chain")]
use async_graphql::{BatchRequest, BatchResponse};
use async_trait::async_trait;
use axum::body::{Body as AxumBody, Bytes};
#[cfg(test)]
use axum::http::HeaderValue;
#[cfg(test)]
use axum::http::header::{ALLOW, CONTENT_TYPE};
use axum::http::{HeaderMap, Response, StatusCode};
#[cfg(test)]
use http_body_util::BodyExt;
use tokio::net::ToSocketAddrs;
use tokio::sync::{Mutex, RwLock, broadcast};
use tokio::task::JoinError;
use tokio::{io, task};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::info;

pub use self::event::{RUES_LOCATION_PREFIX, RuesDispatchEvent, RuesEvent};
pub use error::Error as HttpError;
pub use policy::HttpPolicyConfig;

use self::axum_app::{HttpAppState, build_app};
use self::event::{ResponseData, RuesEventUri, SessionId};
use self::policy::HttpRequestPolicy;
use self::stream::Listener;

pub type HttpResult<T> = std::result::Result<T, HttpError>;

const RUSK_VERSION_HEADER: &str = "Rusk-Version";
const RUSK_VERSION_STRICT_HEADER: &str = "Rusk-Version-Strict";
/// Default cap for most RUES POST request bodies.
pub(crate) const MAX_RUES_REQUEST_BODY_BYTES: usize = 3 * 1024 * 1024;
/// Cap for `POST /on/contract:<id>/upload_driver` request bodies.
pub(crate) const MAX_DRIVER_UPLOAD_BODY_BYTES: usize = 2 * 1024 * 1024;
/// Cap for `POST /graphql` request bodies.
#[cfg(feature = "chain")]
pub(crate) const MAX_GRAPHQL_REQUEST_BODY_BYTES: usize = 256 * 1024;
/// Cap for a single inbound WebSocket message on the RUES subscription socket.
pub(crate) const MAX_WS_INBOUND_MESSAGE_BYTES: usize = 256 * 1024;
/// Cap for a single inbound WebSocket frame payload on the RUES subscription socket.
pub(crate) const MAX_WS_INBOUND_FRAME_BYTES: usize = 64 * 1024;

pub(crate) fn max_rues_request_body_bytes(uri: &RuesEventUri) -> usize {
    match uri.inner() {
        ("contract", Some(_), "upload_driver") => MAX_DRIVER_UPLOAD_BODY_BYTES,
        _ => MAX_RUES_REQUEST_BODY_BYTES,
    }
}

pub struct HttpServer {
    handle: task::JoinHandle<()>,
    _shutdown: broadcast::Sender<Infallible>,
}

pub struct HttpServerConfig {
    pub address: String,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    pub headers: HeaderMap,
    pub ws_event_channel_cap: usize,
    pub policy: HttpPolicyConfig,
}

impl HttpServer {
    pub async fn wait(self) -> Result<(), JoinError> {
        self.handle.await
    }

    pub async fn bind<A, H, P1, P2>(
        handler: H,
        event_receiver: broadcast::Receiver<RuesEvent>,
        ws_event_channel_cap: usize,
        addr: A,
        headers: HeaderMap,
        policy: HttpPolicyConfig,
        cert_and_key: Option<(P1, P2)>,
    ) -> io::Result<(Self, SocketAddr)>
    where
        A: ToSocketAddrs,
        H: HandleRequest,
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let listener = match cert_and_key {
            Some(cert_and_key) => Listener::bind_tls(addr, cert_and_key).await,
            None => Listener::bind(addr).await,
        }?;

        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

        let local_addr = listener.local_addr()?;

        info!("Starting HTTP Listener to {local_addr}");

        let handle = task::spawn(listening_loop(
            handler,
            listener,
            event_receiver,
            shutdown_receiver,
            headers,
            policy,
            ws_event_channel_cap,
        ));

        let server = Self {
            handle,
            _shutdown: shutdown_sender,
        };
        Ok((server, local_addr))
    }
}

#[derive(Default)]
pub struct DataSources {
    pub sources: Vec<Box<dyn HandleRequest>>,
    #[cfg(feature = "chain")]
    graphql: Option<Arc<dyn GraphqlHandler>>,
}

#[cfg(feature = "chain")]
#[async_trait]
pub trait GraphqlHandler: Send + Sync + 'static {
    async fn execute_graphql(&self, request: BatchRequest) -> BatchResponse;
}

impl DataSources {
    #[cfg(feature = "chain")]
    pub(crate) fn set_graphql_handler<T>(&mut self, handler: T)
    where
        T: GraphqlHandler,
    {
        self.graphql = Some(Arc::new(handler));
    }
}

#[async_trait]
impl HandleRequest for DataSources {
    fn can_handle_rues(&self, event: &RuesDispatchEvent) -> bool {
        self.sources.iter().any(|s| s.can_handle_rues(event))
    }

    #[cfg(feature = "chain")]
    fn graphql_handler(&self) -> Option<&dyn GraphqlHandler> {
        self.graphql.as_deref()
    }

    async fn handle_rues(
        &self,
        event: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        info!("Received event at {}", event.uri);
        event.check_rusk_version()?;
        for h in &self.sources {
            if h.can_handle_rues(event) {
                return h.handle_rues(event).await;
            }
        }
        Err(HttpError::Unsupported)
    }
}

async fn listening_loop<H>(
    handler: H,
    listener: Listener,
    events: broadcast::Receiver<RuesEvent>,
    mut shutdown: broadcast::Receiver<Infallible>,
    headers: HeaderMap,
    policy: HttpPolicyConfig,
    ws_event_channel_cap: usize,
) where
    H: HandleRequest,
{
    let app = build_app(HttpAppState {
        sources: Arc::new(handler),
        sockets_map: Arc::new(RwLock::new(HashMap::new())),
        events: Arc::new(Mutex::new(events)),
        shutdown: Arc::new(Mutex::new(shutdown.resubscribe())),
        ws_event_channel_cap,
        policy: Arc::new(HttpRequestPolicy::new(policy)),
        headers: Arc::new(headers),
    });

    // NormalizePathLayer strips trailing slashes so `/graphql/`
    // is served by the `/graphql` route without duplication.
    // It wraps the entire router so normalisation happens before
    // any routing or middleware runs.
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let make_svc =
        axum::ServiceExt::<axum::http::Request<axum::body::Body>>::into_make_service(app);

    let shutdown_signal = async move {
        let _ = shutdown.recv().await;
    };

    let _ = axum::serve(listener, make_svc)
        .with_graceful_shutdown(shutdown_signal)
        .await;
}

// ExecutionError is intentionally large; boxing it would add complexity
// without meaningful benefit here.
#[allow(clippy::result_large_err)]
pub(super) fn response(
    status: StatusCode,
    body: impl Into<Bytes>,
) -> Result<Response<AxumBody>, ExecutionError> {
    Ok(Response::builder()
        .status(status)
        .header(RUSK_VERSION_HEADER, crate::VERSION.as_str())
        .body(AxumBody::from(body.into()))
        .expect("Failed to build response"))
}

#[async_trait]
pub trait HandleRequest: Send + Sync + 'static {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool;
    #[cfg(feature = "chain")]
    fn graphql_handler(&self) -> Option<&dyn GraphqlHandler> {
        None
    }
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::{fs, thread};

    use super::*;

    #[cfg(feature = "chain")]
    use async_graphql::{
        BatchRequest, BatchResponse, Context, EmptyMutation, EmptySubscription,
        Object, Schema,
    };
    use dusk_core::abi::ContractId;
    use event::{BinaryWrapper, RequestData};
    use node_data::events::contract::{ContractEvent, ContractTxEvent};
    use std::net::TcpStream;
    use tungstenite::Message;
    use tungstenite::client;

    /// A [`HandleRequest`] implementation that returns the same data
    struct TestHandle;
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
    impl HandleRequest for TestHandle {
        fn can_handle_rues(&self, _: &RuesDispatchEvent) -> bool {
            true
        }
        async fn handle_rues(
            &self,
            request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            let response = match request.uri.inner() {
                ("test", _, "stream") => {
                    let (sender, rec) = std::sync::mpsc::channel();
                    thread::spawn(move || {
                        for f in STREAMED_DATA.iter() {
                            sender.send(f.to_vec()).unwrap()
                        }
                    });
                    ResponseData::new(rec)
                }
                ("test", _, "echo") => {
                    ResponseData::new(request.data.as_bytes().to_vec())
                }
                ("test", _, "no-content") => ResponseData::new(DataType::None),
                ("contracts", Some(_), _) => ResponseData::new(DataType::None),
                ("test", _, "internal-error") => {
                    return Err(HttpError::internal("sensitive details"));
                }
                ("test", _, "invalid-header") => {
                    ResponseData::new("ok".to_string())
                        .with_header("bad\nheader", "value")
                }
                ("graphql", _, "query") => {
                    ResponseData::new(serde_json::json!({ "data": "ok" }))
                }
                _ => return Err(HttpError::Unsupported),
            };
            Ok(response)
        }
    }

    #[async_trait]
    impl HandleRequest for SlowRuesHandle {
        fn can_handle_rues(&self, _: &RuesDispatchEvent) -> bool {
            true
        }

        async fn handle_rues(
            &self,
            request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            match request.uri.inner() {
                ("test", _, "slow") => {
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

    const EVENT_CHANNEL_CAP: usize = 16;
    const WS_EVENT_CHANNEL_CAP: usize = 2;

    #[derive(Default)]
    struct TestServerOptions {
        cert_and_key: Option<(&'static str, &'static str)>,
        policy: HttpPolicyConfig,
        headers: HeaderMap,
    }

    async fn bind_test_server<H: HandleRequest>(
        handler: H,
    ) -> (HttpServer, SocketAddr, broadcast::Sender<RuesEvent>) {
        bind_test_server_with_options(handler, TestServerOptions::default())
            .await
    }

    async fn bind_test_server_with_headers<H: HandleRequest>(
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

    async fn bind_test_server_with_options<H: HandleRequest>(
        handler: H,
        options: TestServerOptions,
    ) -> (HttpServer, SocketAddr, broadcast::Sender<RuesEvent>) {
        bind_test_server_with_headers_and_tls(
            handler,
            options.headers,
            options.cert_and_key,
            options.policy,
        )
        .await
    }

    async fn bind_test_server_with_headers_and_tls<H: HandleRequest>(
        handler: H,
        headers: HeaderMap,
        cert_and_key: Option<(&'static str, &'static str)>,
        policy: HttpPolicyConfig,
    ) -> (HttpServer, SocketAddr, broadcast::Sender<RuesEvent>) {
        let (event_sender, event_receiver) =
            broadcast::channel(EVENT_CHANNEL_CAP);
        let (_server, local_addr) = HttpServer::bind(
            handler,
            event_receiver,
            WS_EVENT_CHANNEL_CAP,
            "localhost:0",
            headers,
            policy,
            cert_and_key,
        )
        .await
        .expect("Binding the server to the address should succeed");

        (_server, local_addr, event_sender)
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
    async fn configured_headers_are_added_to_policy_forbidden_responses() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test-header", HeaderValue::from_static("test-value"));

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
                    headers,
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
        let header = response
            .headers()
            .get("x-test-header")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(header, "test-value");
    }

    #[tokio::test]
    async fn policy_graphql_class_rate_limit_rejects_with_too_many_requests() {
        let mut policy = HttpPolicyConfig::default();
        policy.global_limits.classes.graphql = policy::HttpPolicyClassLimit {
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
            .post(format!("http://{local_addr}/on/graphql/query"))
            .body("{ ping }")
            .send()
            .await
            .expect("First request should complete");
        assert_eq!(first.status(), StatusCode::OK);

        let second = client
            .post(format!("http://{local_addr}/on/graphql/query"))
            .body("{ ping }")
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
    async fn configured_headers_are_added_to_policy_rate_limited_responses() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test-header", HeaderValue::from_static("test-value"));

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
                    headers,
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
        let header = second
            .headers()
            .get("x-test-header")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(header, "test-value");
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
    async fn post_rues_invalid_path_returns_not_found() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on"))
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::NOT_FOUND,
            "Invalid URL path",
        )
        .await;
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
        let response = responses::request_parse_error_response(
            event::RequestParseError::Other(
                HttpError::internal("sensitive details").into(),
            ),
        )
        .expect("response should be built");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body should be readable")
            .to_bytes();
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
    async fn get_rues_without_session_id_returns_failed_dependency() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let contract_id_hex = hex::encode(ContractId::from_bytes([8; 32]));
        let path =
            format!("http://{local_addr}/on/contracts:{contract_id_hex}/topic");
        let response = client
            .get(path)
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::FAILED_DEPENDENCY,
            "Session ID not provided or invalid",
        )
        .await;
    }

    #[tokio::test]
    async fn get_rues_root_path_without_session_id_returns_failed_dependency() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{local_addr}/on"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::FAILED_DEPENDENCY,
            "Session ID not provided or invalid",
        )
        .await;
    }

    #[tokio::test]
    async fn get_rues_root_path_with_session_id_returns_not_found() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{local_addr}/on"))
            .header("Rusk-Session-Id", "00112233445566778899aabbccddeeff")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::NOT_FOUND,
            "Invalid URL path",
        )
        .await;
    }

    #[tokio::test]
    async fn get_delete_rues_invalid_session_id_returns_failed_dependency() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let contract_id_hex = hex::encode(ContractId::from_bytes([9; 32]));
        let path =
            format!("http://{local_addr}/on/contracts:{contract_id_hex}/topic");

        let get_response = client
            .get(path.clone())
            .header("Rusk-Session-Id", "invalid-session-id")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            get_response,
            StatusCode::FAILED_DEPENDENCY,
            "Session ID not provided or invalid",
        )
        .await;

        let delete_response = client
            .delete(path)
            .header("Rusk-Session-Id", "invalid-session-id")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            delete_response,
            StatusCode::FAILED_DEPENDENCY,
            "Session ID not provided or invalid",
        )
        .await;
    }

    #[tokio::test]
    async fn put_rues_without_session_id_returns_failed_dependency() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let contract_id_hex = hex::encode(ContractId::from_bytes([7; 32]));
        let path =
            format!("http://{local_addr}/on/contracts:{contract_id_hex}/topic");
        let response = client
            .put(path)
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::FAILED_DEPENDENCY,
            "Session ID not provided or invalid",
        )
        .await;
    }

    #[tokio::test]
    async fn put_rues_root_path_with_session_id_returns_not_found() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .put(format!("http://{local_addr}/on"))
            .header("Rusk-Session-Id", "00112233445566778899aabbccddeeff")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_status_contains(
            response,
            StatusCode::NOT_FOUND,
            "Invalid URL path",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn put_rues_returns_method_not_allowed() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;
        let (_stream, sid) = connect_ws(local_addr);

        let client = reqwest::Client::new();
        let contract_id_hex = hex::encode(ContractId::from_bytes([7; 32]));
        let path =
            format!("http://{local_addr}/on/contracts:{contract_id_hex}/topic");
        let response = client
            .put(path)
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        let allow_header = response
            .headers()
            .get(ALLOW)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();

        assert_status_contains(
            response,
            StatusCode::METHOD_NOT_ALLOWED,
            "Method not allowed",
        )
        .await;
        assert_eq!(allow_header, "GET, DELETE");
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
    async fn legacy_graphql_query_still_works() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

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
    async fn graphql_post_query() {
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

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
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

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
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

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
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHttpHeaderHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

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
    async fn graphql_get_query_returns_ok() {
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{local_addr}/graphql?query=%7Bping%7D"))
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
    async fn graphql_get_missing_query_returns_bad_request() {
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{local_addr}/graphql"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_graphql_error_contains(
            response,
            StatusCode::BAD_REQUEST,
            "require a query parameter",
        )
        .await;
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_put_returns_method_not_allowed_with_allow_header() {
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

        let client = reqwest::Client::new();
        let response = client
            .put(format!("http://{local_addr}/graphql"))
            .send()
            .await
            .expect("Requesting should succeed");

        let allow_header = response
            .headers()
            .get(ALLOW)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();

        assert_graphql_error_contains(
            response,
            StatusCode::METHOD_NOT_ALLOWED,
            "Method not allowed",
        )
        .await;
        assert_eq!(allow_header, "GET, POST");
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_not_configured_returns_not_found() {
        let handler = DataSources::default();
        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{local_addr}/graphql?query=%7Bping%7D"))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_graphql_error_contains(
            response,
            StatusCode::NOT_FOUND,
            "GraphQL endpoint not configured",
        )
        .await;
    }

    #[tokio::test]
    #[cfg(feature = "chain")]
    async fn graphql_trailing_slash_path_works() {
        let mut handler = DataSources::default();
        handler.set_graphql_handler(TestGraphqlHandler);

        let (_server, local_addr, _event_sender) =
            bind_test_server(handler).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/graphql/"))
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
    #[cfg(feature = "http-wasm")]
    async fn http_wasm_wallet_core_alias_returns_wasm_with_cache_headers() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!(
                "http://{local_addr}/static/drivers/wallet-core.wasm"
            ))
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let cache_control = response
            .headers()
            .get("Cache-Control")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = response
            .bytes()
            .await
            .expect("Reading response body should succeed");

        assert_eq!(content_type, "application/wasm");
        assert_eq!(cache_control, "public, max-age=31536000, immutable");
        assert!(!body.is_empty(), "WASM response body should not be empty");
    }

    #[tokio::test]
    #[cfg(feature = "http-wasm")]
    async fn http_wasm_versioned_paths_return_wasm() {
        let (_server, local_addr, _event_sender) =
            bind_test_server(TestHandle).await;

        let paths = [
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

            assert_eq!(response.status(), StatusCode::OK);
            let content_type = response
                .headers()
                .get("Content-Type")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let cache_control = response
                .headers()
                .get("Cache-Control")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let body = response
                .bytes()
                .await
                .expect("Reading response body should succeed");

            assert_eq!(content_type, "application/wasm");
            assert_eq!(cache_control, "public, max-age=31536000, immutable");
            assert!(
                !body.is_empty(),
                "WASM response body should not be empty for {path}"
            );
        }
    }

    #[tokio::test]
    async fn configured_headers_are_added_to_success_responses() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test-header", HeaderValue::from_static("test-value"));

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_headers(TestHandle, headers).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/echo"))
            .body("hello")
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let header = response
            .headers()
            .get("x-test-header")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(header, "test-value");
    }

    #[tokio::test]
    async fn configured_headers_are_added_to_error_responses() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test-header", HeaderValue::from_static("test-value"));

        let (_server, local_addr, _event_sender) =
            bind_test_server_with_headers(TestHandle, headers).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{local_addr}/on/test/unsupported"))
            .send()
            .await
            .expect("Requesting should succeed");

        let header = response
            .headers()
            .get("x-test-header")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert_eq!(header, "test-value");

        assert_status_contains(
            response,
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

        let uri = RuesEventUri::parse_from_path(&path)
            .ok_or(anyhow::anyhow!("Invalid location"))?;

        let data = data.to_vec().into();
        Ok(RuesEvent { data, headers, uri })
    }
}
