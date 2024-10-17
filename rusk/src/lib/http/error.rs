// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Response Errors for server responses

use async_graphql::ServerError;
use hyper::StatusCode;
use thiserror::Error;
use tracing::error;

pub type PublicResult<T> = std::result::Result<T, PublicError>;

/// Error Type that can map to HTTP status codes and be passed to the API
/// consumer with the error message
#[derive(Debug, Error)]
pub enum PublicError {
    #[error("Failed serializing response: {:?}", message)]
    Serialize { message: String }, // source: serde_json::Error
    #[error("Invalid URL path")]
    InvalidPath,
    #[error("Invalid request body data")]
    InvalidBodyData,
    #[error("Invalid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Not enough bytes for parsed length: {len}, minimum required: {required}")]
    ParseBytes { len: usize, required: usize },
    #[error("Not enough bytes for target type")]
    ParseTargetType,
    #[error("{0}")]
    HandleRequest(#[from] crate::http::RequestError),

    /*
     * Previously ExecutionErrors
     */
    #[error("{0}")]
    Http(#[from] hyper::http::Error),
    #[error("{0}")]
    Hyper(#[from] hyper::Error),
    #[error("Failed parsing JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Protocol(#[from] tungstenite::error::ProtocolError),
    #[error("{0}")]
    Websocket(#[from] tungstenite::Error),
    /*
     * Previously ExecutionError::Generic(anyhow::Error)
     */
    #[error("{0}")]
    InvalidHeaderValue(#[from] hyper::header::InvalidHeaderValue),
    #[error("{0}")]
    InvalidHeaderName(#[from] hyper::header::InvalidHeaderName),
}

impl PublicError {
    /// Get the HTTP status code for the error
    ///
    /// # Dev Note
    ///
    /// There should never be a _ match arm here.
    /// New errors which can appear should be propagated up to `PublicError` and
    /// need to have a statuscode explicitly defined here.
    ///
    /// Having a catch-all arm would introduce the possibility of a new
    /// error being introduced and not having a proper status code defined for
    /// it, because the code compiles and runs without error.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            PublicError::ParseBytes { .. }
            | PublicError::ParseTargetType
            | PublicError::InvalidBodyData => StatusCode::BAD_REQUEST,
            PublicError::Utf8(_) => StatusCode::UNPROCESSABLE_ENTITY,
            PublicError::InvalidHeaderValue(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            PublicError::InvalidHeaderName(_) => {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            }
            PublicError::InvalidPath => StatusCode::NOT_FOUND,
            PublicError::Http(_)
            | PublicError::Hyper(_)
            | PublicError::Protocol(_)
            | PublicError::Json(_)
            | PublicError::Serialize { .. }
            | PublicError::Websocket(_) => {
                error!("Internal Server Error: {:?}", self);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            PublicError::HandleRequest(request_error) => request_error.status(),
        }
    }
}
