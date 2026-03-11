// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use axum::http::StatusCode;
use axum::http::header::{ALLOW, CONTENT_TYPE};
use axum::http::{HeaderValue, Response};
use axum::response::IntoResponse;
use tracing::{debug, error};

use super::event::ExecutionError;
use super::event::RequestParseError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Client provided invalid input (malformed hex, bad contract ID, etc.)
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// Version mismatch or missing version header
    #[error("Version error: {0}")]
    VersionMismatch(String),
    /// Invalid UTF-8 in request body
    #[error("Invalid request encoding: {0}")]
    InvalidEncoding(String),
    /// Request payload too large
    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),
    /// Requested resource was not found
    #[error("Not found: {0}")]
    NotFound(String),
    /// Request blocked by ACL policy
    #[error("Forbidden: {0}")]
    Forbidden(String),
    /// Request rejected by rate/concurrency limiting
    #[error("Too many requests: {0}")]
    TooManyRequests(String),
    /// Unsupported operation / endpoint
    #[error("Unsupported operation")]
    Unsupported,
    /// JSON serialization/deserialization failure
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// VM / contract execution error
    #[error("VM error: {0}")]
    Vm(String),
    /// Database / storage error
    #[error("Database error: {0}")]
    Database(String),
    /// Data driver encode/decode error
    #[error("Data driver error: {0}")]
    DataDriver(String),
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Prover error
    #[error("Prover error: {0}")]
    Prover(String),
    /// Signature / cryptographic verification failure
    #[error("Verification error: {0}")]
    Verification(String),
    /// Catch-all for errors that don't fit other variants
    #[error("{0}")]
    Internal(String),
}

impl Error {
    pub fn http_code(&self) -> u16 {
        match self {
            Error::InvalidInput(_)
            | Error::VersionMismatch(_)
            | Error::InvalidEncoding(_) => 400,
            Error::PayloadTooLarge(_) => 413,
            Error::NotFound(_) => 404,
            Error::Forbidden(_) => 403,
            Error::TooManyRequests(_) => 429,
            // TODO: Keep 501 for current compatibility; revisit whether
            // unsupported routes/operations should be normalized to 404.
            Error::Unsupported => 501,
            Error::Serialization(_)
            | Error::Vm(_)
            | Error::Database(_)
            | Error::DataDriver(_)
            | Error::Io(_)
            | Error::Prover(_)
            | Error::Verification(_)
            | Error::Internal(_) => 500,
        }
    }

    pub fn invalid_input<T: AsRef<str>>(msg: T) -> Self {
        Error::InvalidInput(msg.as_ref().to_string())
    }

    pub fn not_found<T: AsRef<str>>(msg: T) -> Self {
        Error::NotFound(msg.as_ref().to_string())
    }

    pub fn vm<T: AsRef<str>>(msg: T) -> Self {
        Error::Vm(msg.as_ref().to_string())
    }

    pub fn forbidden<T: AsRef<str>>(msg: T) -> Self {
        Error::Forbidden(msg.as_ref().to_string())
    }

    pub fn too_many_requests<T: AsRef<str>>(msg: T) -> Self {
        Error::TooManyRequests(msg.as_ref().to_string())
    }

    pub fn database<T: AsRef<str>>(msg: T) -> Self {
        Error::Database(msg.as_ref().to_string())
    }

    pub fn data_driver<T: AsRef<str>>(msg: T) -> Self {
        Error::DataDriver(msg.as_ref().to_string())
    }

    pub fn prover<T: AsRef<str>>(msg: T) -> Self {
        Error::Prover(msg.as_ref().to_string())
    }

    pub fn verification<T: AsRef<str>>(msg: T) -> Self {
        Error::Verification(msg.as_ref().to_string())
    }

    pub fn internal<T: AsRef<str>>(msg: T) -> Self {
        Error::Internal(msg.as_ref().to_string())
    }

    pub fn payload_too_large<T: AsRef<str>>(msg: T) -> Self {
        Error::PayloadTooLarge(msg.as_ref().to_string())
    }
}

impl From<dusk_data_driver::Error> for Error {
    fn from(e: dusk_data_driver::Error) -> Self {
        Self::DataDriver(e.to_string())
    }
}

impl From<semver::Error> for Error {
    fn from(e: semver::Error) -> Self {
        Self::VersionMismatch(e.to_string())
    }
}

pub(super) fn map_http_error_for_response(error: &Error) -> (u16, String) {
    let status = error.http_code();
    let message = match error {
        Error::InvalidInput(_)
        | Error::VersionMismatch(_)
        | Error::InvalidEncoding(_)
        | Error::PayloadTooLarge(_)
        | Error::NotFound(_)
        | Error::Forbidden(_)
        | Error::TooManyRequests(_)
        | Error::Unsupported => error.to_string(),
        Error::Serialization(_)
        | Error::Vm(_)
        | Error::Database(_)
        | Error::DataDriver(_)
        | Error::Io(_)
        | Error::Prover(_)
        | Error::Verification(_)
        | Error::Internal(_) => "Internal server error".to_string(),
    };
    (status, message)
}

#[derive(Debug, Clone)]
pub(super) struct ApiError {
    status: StatusCode,
    message: String,
    category: &'static str,
    allow: Option<&'static str>,
    retry_after_seconds: Option<u64>,
}

impl ApiError {
    pub(super) fn new(
        status: StatusCode,
        message: impl Into<String>,
        category: &'static str,
    ) -> Self {
        Self {
            status,
            message: message.into(),
            category,
            allow: None,
            retry_after_seconds: None,
        }
    }

    pub(super) fn method_not_allowed(allow: &'static str) -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            message: "Method not allowed".to_string(),
            category: "method_not_allowed",
            allow: Some(allow),
            retry_after_seconds: None,
        }
    }

    pub(super) fn with_retry_after(mut self, retry_after_seconds: u64) -> Self {
        self.retry_after_seconds = Some(retry_after_seconds.max(1));
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response<axum::body::Body> {
        if self.status.is_server_error() {
            error!(
                status = %self.status,
                error_category = self.category,
                error_message = %self.message,
                "HTTP request failed"
            );
        } else {
            debug!(
                status = %self.status,
                error_category = self.category,
                error_message = %self.message,
                "HTTP request rejected"
            );
        }
        let mut response = super::response(
            self.status,
            serde_json::json!({ "error": self.message }).to_string(),
        )
        .expect("API error response should be buildable");
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(allow) = self.allow {
            response
                .headers_mut()
                .insert(ALLOW, HeaderValue::from_static(allow));
        }
        if let Some(retry_after) = self.retry_after_seconds
            && let Ok(value) = HeaderValue::from_str(&retry_after.to_string())
        {
            response.headers_mut().insert("Retry-After", value);
        }
        response
    }
}

impl From<Error> for ApiError {
    fn from(value: Error) -> Self {
        let (status, message) = map_http_error_for_response(&value);
        let status = StatusCode::from_u16(status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            message,
            category: http_error_category(&value),
            allow: None,
            retry_after_seconds: None,
        }
    }
}

impl From<ExecutionError> for ApiError {
    fn from(value: ExecutionError) -> Self {
        let (status, message, category) = map_execution_error(&value);
        Self {
            status,
            message,
            category,
            allow: None,
            retry_after_seconds: None,
        }
    }
}

impl From<RequestParseError> for ApiError {
    fn from(value: RequestParseError) -> Self {
        match value {
            RequestParseError::InvalidPath => Self::new(
                StatusCode::NOT_FOUND,
                "Invalid URL path",
                "invalid_path",
            ),
            RequestParseError::InvalidPayload(message) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                message,
                "invalid_payload",
            ),
            RequestParseError::Other(err) => {
                if let Some(http_err) = err.downcast_ref::<Error>() {
                    let (status, message) =
                        map_http_error_for_response(http_err);
                    let status = StatusCode::from_u16(status)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                    return Self {
                        status,
                        message,
                        category: http_error_category(http_err),
                        allow: None,
                        retry_after_seconds: None,
                    };
                }
                error!(error = %err, "Failed parsing RUES dispatch request");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed parsing request",
                    "internal",
                )
            }
        }
    }
}

pub(super) fn map_execution_error(
    error: &ExecutionError,
) -> (StatusCode, String, &'static str) {
    match error {
        ExecutionError::Http(_)
        | ExecutionError::Json(_)
        | ExecutionError::Protocol(_)
        | ExecutionError::Tungstenite(_)
        | ExecutionError::Other(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
            "internal",
        ),
        ExecutionError::InvalidHeader(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid header".to_string(),
            "invalid_header",
        ),
    }
}

pub(super) fn http_error_category(error: &Error) -> &'static str {
    match error {
        Error::InvalidInput(_) => "invalid_input",
        Error::VersionMismatch(_) => "version_mismatch",
        Error::InvalidEncoding(_) => "invalid_encoding",
        Error::PayloadTooLarge(_) => "payload_too_large",
        Error::NotFound(_) => "not_found",
        Error::Forbidden(_) => "forbidden",
        Error::TooManyRequests(_) => "too_many_requests",
        Error::Unsupported => "unsupported",
        Error::Serialization(_) => "serialization",
        Error::Vm(_) => "vm",
        Error::Database(_) => "database",
        Error::DataDriver(_) => "data_driver",
        Error::Io(_) => "io",
        Error::Prover(_) => "prover",
        Error::Verification(_) => "verification",
        Error::Internal(_) => "internal",
    }
}
