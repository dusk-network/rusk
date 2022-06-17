// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Display;
use std::task::{Context, Poll};

use hyper::header::HeaderValue;
use hyper::Body;
use semver::{Version, VersionReq};
use tonic::service::Interceptor;
use tonic::{body::BoxBody, Status};
use tower::{Layer, Service};
use tracing::{error, info};

/// Rusk version
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Default)]
pub struct RuskVersionLayer;

impl<S> Layer<S> for RuskVersionLayer {
    type Service = RuskVersionMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        RuskVersionMiddleware { inner: service }
    }
}

/// This middleware adds `x-rusk-version` to response headers
/// for any server response, be it the result of a sucessful
/// request or not.
#[derive(Debug, Clone)]
pub struct RuskVersionMiddleware<S> {
    inner: S,
}

impl<S> Service<hyper::Request<Body>> for RuskVersionMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Display,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<
        'static,
        Result<Self::Response, Self::Error>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: hyper::Request<Body>) -> Self::Future {
        // This is necessary because tonic internally uses
        // `tower::buffer::Buffer`. See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let xray = uuid::Uuid::new_v4();
        let path = req.uri().path().to_owned();
        info!("[{}] {} - received request", xray, path);

        Box::pin(async move {
            let response = inner.call(req).await;
            match response {
                Ok(mut response) => {
                    response.headers_mut().append(
                        "x-rusk-version",
                        HeaderValue::from_str(VERSION).unwrap(),
                    );
                    info!("[{}] {} - OK", xray, path);
                    Ok(response)
                }
                Err(e) => {
                    error!("[{}] {} - ERROR {}", xray, path, e);
                    Err(e)
                }
            }
        })
    }
}

/// Checks incoming requests for compatibility with running
/// Rusk version using an Interceptor.
#[derive(Clone)]
pub struct CompatibilityInterceptor;

impl Interceptor for CompatibilityInterceptor {
    fn call(
        &mut self,
        request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, Status> {
        // attempt to extract `x-rusk-version` header metadata
        let metadata = request.metadata();
        match metadata.get("x-rusk-version") {
            Some(header_v) => {
                // extract a string value
                let mut client_version = "unknown";
                let is_compat = match header_v.to_str() {
                    Ok(req_v) => {
                        // check for compatibility
                        client_version = req_v;
                        is_compatible(req_v, VERSION)
                    }
                    Err(_) => false,
                };
                if !is_compat {
                    return Err(Status::failed_precondition(
                        format!("Requested rusk version is not supported. Expected {} but got {}!", VERSION, client_version),
                    ));
                }
            }
            None => {
                return Err(Status::unavailable(
                    "Missing \"x-rusk-version\" header, please update client!",
                ))
            }
        }
        Ok(request)
    }
}

/// Returns true if `client_version` is compatible with
/// current crate version.
///
/// Compatibility is defined according to basic semver
/// rules. If an error occurs or there's no version present
/// the function will return false.
fn is_compatible(req_v: &str, rusk_v: &'static str) -> bool {
    let req_v = Version::parse(req_v);
    let req = VersionReq::parse(rusk_v);
    match (req, req_v) {
        (Ok(req), Ok(req_v)) => req.matches(&req_v),
        _ => false,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn semver_test() {
        let vr = VersionReq::parse("0.4").unwrap();
        let v = Version::parse("0.5.0").unwrap();
        assert!(!vr.matches(&v));

        let vr = VersionReq::parse("0.4.0-rc.1").unwrap();
        let v = Version::parse("0.4.1").unwrap();
        assert!(vr.matches(&v));
    }

    #[test]
    fn compatibility_test() {
        assert!(is_compatible("0.5.0-rc.0", "0.5.0-rc.0"));
        assert!(is_compatible("0.4.0", "0.4.0"));
        assert!(is_compatible("4.2.0", "4.1.0"));
        assert!(!is_compatible("0.4.0", "0.5.1"));
        assert!(!is_compatible("0.4.0", "4.0.0"));
        assert!(!is_compatible("4.0.0", "0.5.0"));
    }
}
