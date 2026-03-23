// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::{Ready, ready};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

use axum::http::{HeaderMap, Request, StatusCode};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tower::Service;
use tower::limit::rate::{Rate, RateLimit};
use tracing::warn;

use super::RUES_LOCATION_PREFIX;
use super::event::RuesEventUri;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpPolicyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub acl: HttpPolicyAclConfig,
    #[serde(default)]
    pub global_limits: HttpPolicyGlobalLimitsConfig,
}

impl Default for HttpPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            acl: HttpPolicyAclConfig::default(),
            global_limits: HttpPolicyGlobalLimitsConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpPolicyAclConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub default_action: HttpPolicyAclDefaultAction,
    #[serde(default)]
    pub rules: Vec<HttpPolicyAclRule>,
}

impl Default for HttpPolicyAclConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_action: HttpPolicyAclDefaultAction::Allow,
            rules: Vec::new(),
        }
    }
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum HttpPolicyAclDefaultAction {
    #[default]
    Allow,
    Deny,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HttpPolicyAclAction {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpPolicyAclRule {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub action: HttpPolicyAclAction,
    pub path: String,
    #[serde(default)]
    pub method: Vec<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpPolicyGlobalLimitsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub classes: HttpPolicyClassLimitsConfig,
}

impl Default for HttpPolicyGlobalLimitsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            classes: HttpPolicyClassLimitsConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpPolicyClassLimitsConfig {
    #[serde(default = "default_contract_query_limit")]
    pub contract_query: HttpPolicyClassLimit,
    #[serde(default = "default_feeder_query_limit")]
    pub feeder_query: HttpPolicyClassLimit,
    #[serde(default = "default_graphql_limit")]
    pub graphql: HttpPolicyClassLimit,
    #[serde(default = "default_tx_propagate_limit")]
    pub tx_propagate: HttpPolicyClassLimit,
    #[serde(default = "default_upload_driver_limit")]
    pub upload_driver: HttpPolicyClassLimit,
    #[serde(default = "default_other_rues_limit")]
    pub other_rues: HttpPolicyClassLimit,
    #[serde(default = "default_other_http_limit")]
    pub other_http: HttpPolicyClassLimit,
}

impl Default for HttpPolicyClassLimitsConfig {
    fn default() -> Self {
        Self {
            contract_query: default_contract_query_limit(),
            feeder_query: default_feeder_query_limit(),
            graphql: default_graphql_limit(),
            tx_propagate: default_tx_propagate_limit(),
            upload_driver: default_upload_driver_limit(),
            other_rues: default_other_rues_limit(),
            other_http: default_other_http_limit(),
        }
    }
}

impl HttpPolicyClassLimitsConfig {
    /// Converts class limits into a stable class-indexed array.
    fn into_array(self) -> [HttpPolicyClassLimit; ENDPOINT_CLASS_COUNT] {
        [
            self.contract_query,
            self.feeder_query,
            self.graphql,
            self.tx_propagate,
            self.upload_driver,
            self.other_rues,
            self.other_http,
        ]
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct HttpPolicyClassLimit {
    pub rps: u32,
    pub burst: u32,
    pub concurrency: usize,
}

pub struct HttpRequestPolicy {
    enabled: bool,
    acl: Option<AclEngine>,
    limits: Option<GlobalLimiter>,
}

impl HttpRequestPolicy {
    /// Builds a request policy from HTTP policy configuration.
    pub fn new(config: HttpPolicyConfig) -> Self {
        if !config.enabled {
            return Self {
                enabled: false,
                acl: None,
                limits: None,
            };
        }

        let acl = config.acl.enabled.then(|| AclEngine::new(config.acl));
        let limits = config
            .global_limits
            .enabled
            .then(|| GlobalLimiter::new(config.global_limits.classes));

        Self {
            enabled: true,
            acl,
            limits,
        }
    }

    /// Enforces ACL and endpoint-class limits for an incoming HTTP request.
    ///
    /// Returns a permit that must be held for the lifetime of request
    /// execution, or a policy rejection that should be returned to the client.
    pub fn enforce<B>(
        &self,
        req: &Request<B>,
    ) -> Result<PolicyPermit, PolicyRejection> {
        if !self.enabled {
            return Ok(PolicyPermit::none());
        }

        let path = req.uri().path();

        if path.starts_with(RUES_LOCATION_PREFIX)
            && self.acl.as_ref().is_some_and(|acl| acl.is_denied(req))
        {
            return Err(PolicyRejection::forbidden());
        }

        let class = classify_request(path, req.headers());

        if let Some(limits) = &self.limits {
            return limits.acquire(class, path);
        }

        Ok(PolicyPermit::none())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EndpointClass {
    ContractQuery,
    FeederQuery,
    Graphql,
    TxPropagate,
    UploadDriver,
    OtherRues,
    OtherHttp,
}

impl EndpointClass {
    const ALL: [Self; ENDPOINT_CLASS_COUNT] = [
        Self::ContractQuery,
        Self::FeederQuery,
        Self::Graphql,
        Self::TxPropagate,
        Self::UploadDriver,
        Self::OtherRues,
        Self::OtherHttp,
    ];

    /// Returns a stable array index for internal class tables.
    const fn as_index(self) -> usize {
        match self {
            Self::ContractQuery => 0,
            Self::FeederQuery => 1,
            Self::Graphql => 2,
            Self::TxPropagate => 3,
            Self::UploadDriver => 4,
            Self::OtherRues => 5,
            Self::OtherHttp => 6,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EndpointClass::ContractQuery => "contract_query",
            EndpointClass::FeederQuery => "feeder_query",
            EndpointClass::Graphql => "graphql",
            EndpointClass::TxPropagate => "tx_propagate",
            EndpointClass::UploadDriver => "upload_driver",
            EndpointClass::OtherRues => "other_rues",
            EndpointClass::OtherHttp => "other_http",
        }
    }
}

const ENDPOINT_CLASS_COUNT: usize = 7;

/// Classifies an HTTP request path into a policy endpoint class.
///
/// For RUES paths, this operates on the `target/topic` route where `target` is
/// internally parsed as `component[:entity]`.
pub fn classify_request(path: &str, headers: &HeaderMap) -> EndpointClass {
    if is_graphql_path(path) {
        return EndpointClass::Graphql;
    }

    if !path.starts_with(RUES_LOCATION_PREFIX) {
        return EndpointClass::OtherHttp;
    }

    let Some(uri) = RuesEventUri::parse_from_path(path) else {
        return EndpointClass::OtherRues;
    };

    match uri.inner() {
        ("graphql", _, "query") => EndpointClass::Graphql,
        ("contracts", Some(_), _) => {
            if headers.contains_key("Rusk-Feeder") {
                EndpointClass::FeederQuery
            } else {
                EndpointClass::ContractQuery
            }
        }
        ("transactions", _, "propagate") => EndpointClass::TxPropagate,
        ("contract", Some(_), "upload_driver") => EndpointClass::UploadDriver,
        _ => EndpointClass::OtherRues,
    }
}

pub struct PolicyPermit {
    _permit: Option<OwnedSemaphorePermit>,
}

impl PolicyPermit {
    fn none() -> Self {
        Self { _permit: None }
    }

    fn from_permit(permit: OwnedSemaphorePermit) -> Self {
        Self {
            _permit: Some(permit),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PolicyRejection {
    pub status: StatusCode,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

impl PolicyRejection {
    fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: "forbidden".to_string(),
            retry_after_seconds: None,
        }
    }

    fn too_many_requests(retry_after_seconds: u64) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "too_many_requests".to_string(),
            retry_after_seconds: Some(retry_after_seconds.max(1)),
        }
    }
}

#[derive(Clone)]
struct AclEngine {
    default_action: HttpPolicyAclDefaultAction,
    rules: Arc<Vec<CompiledAclRule>>,
}

#[derive(Clone)]
struct CompiledAclRule {
    id: String,
    action: HttpPolicyAclAction,
    path_regex: Regex,
    methods: Vec<String>,
    headers: Vec<(String, String)>,
}

impl AclEngine {
    fn new(config: HttpPolicyAclConfig) -> Self {
        let rules = config
            .rules
            .into_iter()
            .filter(|rule| rule.enabled)
            .map(|rule| CompiledAclRule {
                id: rule.id,
                action: rule.action,
                path_regex: wildcard_pattern_to_regex(
                    &rule.path.to_ascii_lowercase(),
                ),
                methods: rule
                    .method
                    .into_iter()
                    .map(|m| m.to_ascii_uppercase())
                    .collect(),
                headers: rule
                    .headers
                    .into_iter()
                    .map(|(k, v)| (k.to_ascii_lowercase(), v))
                    .collect(),
            })
            .collect();

        Self {
            default_action: config.default_action,
            rules: Arc::new(rules),
        }
    }

    /// Evaluates ACL rules and returns `true` when request must be denied.
    fn is_denied<B>(&self, req: &Request<B>) -> bool {
        let path = req.uri().path().to_ascii_lowercase();
        let method = req.method().as_str().to_ascii_uppercase();

        for rule in self.rules.iter() {
            if !rule.path_regex.is_match(&path) {
                continue;
            }

            if !rule.methods.is_empty()
                && !rule.methods.iter().any(|m| m == &method)
            {
                continue;
            }

            let headers_ok = rule.headers.iter().all(|(name, expected)| {
                req.headers()
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|actual| actual == expected)
            });
            if !headers_ok {
                continue;
            }

            if matches!(rule.action, HttpPolicyAclAction::Deny) {
                warn!(
                    counter = "acl_denied_total",
                    rule_id = %rule.id,
                    path = %req.uri().path(),
                    "HTTP policy denied request"
                );
                return true;
            }
            return false;
        }

        matches!(self.default_action, HttpPolicyAclDefaultAction::Deny)
    }
}

#[derive(Clone)]
struct GlobalLimiter {
    classes: [ClassLimiter; ENDPOINT_CLASS_COUNT],
}

impl GlobalLimiter {
    fn new(config: HttpPolicyClassLimitsConfig) -> Self {
        let mut limits = config.into_array();
        for (index, limit) in limits.iter_mut().enumerate() {
            sanitize_limit(limit, EndpointClass::ALL[index]);
        }

        Self {
            classes: limits.map(ClassLimiter::new),
        }
    }

    fn limiter(&self, class: EndpointClass) -> &ClassLimiter {
        &self.classes[class.as_index()]
    }

    /// Acquires rate and concurrency budget for a specific endpoint class.
    fn acquire(
        &self,
        class: EndpointClass,
        path: &str,
    ) -> Result<PolicyPermit, PolicyRejection> {
        let limiter = self.limiter(class);
        match limiter.acquire() {
            Ok(permit) => Ok(PolicyPermit::from_permit(permit)),
            Err(LimiterReject::RateLimited(retry_after_seconds)) => {
                warn!(
                    counter = "rate_limited_total",
                    class = class.as_str(),
                    path,
                    retry_after_seconds,
                    "HTTP policy rate-limit rejection"
                );
                Err(PolicyRejection::too_many_requests(retry_after_seconds))
            }
            Err(LimiterReject::ConcurrencyLimited) => {
                warn!(
                    counter = "concurrency_limited_total",
                    class = class.as_str(),
                    path,
                    "HTTP policy concurrency-limit rejection"
                );
                Err(PolicyRejection::too_many_requests(1))
            }
        }
    }
}

#[derive(Clone)]
struct ClassLimiter {
    rate_limiter: Arc<Mutex<RateLimit<NoopRateService>>>,
    retry_after_seconds: u64,
    semaphore: Arc<Semaphore>,
}

impl ClassLimiter {
    fn new(config: HttpPolicyClassLimit) -> Self {
        let rps = config.rps.max(1) as f64;
        let burst = config.burst.max(1) as u64;
        let window = Duration::from_secs_f64((burst as f64 / rps).max(0.001));
        let retry_after_seconds = (1.0 / rps).ceil() as u64;

        Self {
            rate_limiter: Arc::new(Mutex::new(RateLimit::new(
                NoopRateService,
                Rate::new(burst, window),
            ))),
            retry_after_seconds: retry_after_seconds.max(1),
            semaphore: Arc::new(Semaphore::new(config.concurrency)),
        }
    }

    /// Tries to consume one token and one concurrency slot.
    fn acquire(&self) -> Result<OwnedSemaphorePermit, LimiterReject> {
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| LimiterReject::ConcurrencyLimited)?;

        let mut limiter = self
            .rate_limiter
            .lock()
            .expect("Policy rate limiter mutex should be lockable");

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        match limiter.poll_ready(&mut cx) {
            Poll::Ready(Ok(())) => {
                drop(limiter.call(()));
                Ok(permit)
            }
            Poll::Pending => {
                drop(permit);
                Err(LimiterReject::RateLimited(self.retry_after_seconds))
            }
            Poll::Ready(Err(_)) => {
                drop(permit);
                Err(LimiterReject::RateLimited(self.retry_after_seconds))
            }
        }
    }
}

enum LimiterReject {
    RateLimited(u64),
    ConcurrencyLimited,
}

#[derive(Clone, Copy, Debug, Default)]
struct NoopRateService;

impl Service<()> for NoopRateService {
    type Response = ();
    type Error = Infallible;
    type Future = Ready<Result<(), Infallible>>;

    fn poll_ready(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: ()) -> Self::Future {
        ready(Ok(()))
    }
}

#[derive(Default)]
struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn noop_waker() -> Waker {
    Waker::from(Arc::new(NoopWake))
}

/// Ensures class limits always remain valid at runtime.
fn sanitize_limit(limit: &mut HttpPolicyClassLimit, class: EndpointClass) {
    if limit.rps == 0 {
        warn!(
            class = class.as_str(),
            "Invalid HTTP policy class rps=0, coercing to 1"
        );
        limit.rps = 1;
    }

    if limit.burst == 0 {
        warn!(
            class = class.as_str(),
            "Invalid HTTP policy class burst=0, coercing to 1"
        );
        limit.burst = 1;
    }

    if limit.concurrency == 0 {
        warn!(
            class = class.as_str(),
            "Invalid HTTP policy class concurrency=0, coercing to 1"
        );
        limit.concurrency = 1;
    }
}

/// Compiles a lowercase wildcard path pattern to a regex.
///
/// `*` is mapped to `.*` so matching behavior remains unchanged.
fn wildcard_pattern_to_regex(pattern: &str) -> Regex {
    let mut escaped = regex::escape(pattern);
    escaped = escaped.replace("\\*", ".*");
    let regex = format!("^{escaped}$");
    Regex::new(&regex)
        .expect("Wildcard pattern should always compile into regex")
}

fn default_true() -> bool {
    true
}

const fn default_contract_query_limit() -> HttpPolicyClassLimit {
    HttpPolicyClassLimit {
        rps: 180,
        burst: 360,
        concurrency: 12,
    }
}

const fn default_feeder_query_limit() -> HttpPolicyClassLimit {
    HttpPolicyClassLimit {
        rps: 3,
        burst: 6,
        concurrency: 3,
    }
}

const fn default_graphql_limit() -> HttpPolicyClassLimit {
    HttpPolicyClassLimit {
        rps: 1500,
        burst: 3000,
        concurrency: 64,
    }
}

const fn default_tx_propagate_limit() -> HttpPolicyClassLimit {
    HttpPolicyClassLimit {
        rps: 400,
        burst: 800,
        concurrency: 32,
    }
}

const fn default_upload_driver_limit() -> HttpPolicyClassLimit {
    HttpPolicyClassLimit {
        rps: 2,
        burst: 4,
        concurrency: 2,
    }
}

const fn default_other_rues_limit() -> HttpPolicyClassLimit {
    HttpPolicyClassLimit {
        rps: 500,
        burst: 1000,
        concurrency: 64,
    }
}

const fn default_other_http_limit() -> HttpPolicyClassLimit {
    HttpPolicyClassLimit {
        rps: 2000,
        burst: 4000,
        concurrency: 128,
    }
}

fn is_graphql_path(path: &str) -> bool {
    matches!(path, "/graphql" | "/graphql/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_pattern_to_regex_handles_edge_cases() {
        let matches = |pattern: &str, path: &str| {
            wildcard_pattern_to_regex(pattern).is_match(path)
        };

        assert!(matches("", ""));
        assert!(!matches("", "/on/test/echo"));

        assert!(matches("*", ""));
        assert!(matches("*", "/on/test/echo"));

        assert!(matches("/on/*", "/on/test"));
        assert!(matches("/on/*/echo", "/on/test/echo"));
        assert!(matches("/on/*/echo", "/on/test/inner/echo"));
        assert!(matches("/on/*/echo", "/on//echo"));
        assert!(matches("/on/**/echo", "/on/test/echo"));

        assert!(!matches("/on/*/echo", "/off/test/echo"));
        assert!(!matches("/on/*/echo", "/on/test/stream"));
    }

    #[tokio::test]
    async fn global_limiter_concurrency_rejection_returns_too_many_requests() {
        let limits = HttpPolicyClassLimitsConfig {
            contract_query: HttpPolicyClassLimit {
                rps: 2,
                burst: 2,
                concurrency: 1,
            },
            ..HttpPolicyClassLimitsConfig::default()
        };
        let limiter = GlobalLimiter::new(limits);

        let first = limiter
            .acquire(EndpointClass::ContractQuery, "/on/contracts:abcd/query")
            .expect("First acquire should succeed");
        let second = match limiter
            .acquire(EndpointClass::ContractQuery, "/on/contracts:abcd/query")
        {
            Ok(_) => {
                panic!("Second acquire should be rejected by concurrency cap")
            }
            Err(err) => err,
        };

        assert_eq!(second.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(second.retry_after_seconds, Some(1));
        assert_eq!(second.message, "too_many_requests");

        drop(first);
    }
}
