// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use axum::body::Body as AxumBody;
use axum::http::header::{ALLOW, CONTENT_TYPE};
use axum::http::{HeaderValue, Response, StatusCode};
use tracing::{debug, error};

use super::error::map_http_error_for_response;
use super::event::RequestParseError;
use super::{ExecutionError, HttpError, response};
use crate::http::event::FullOrStreamBody;

// ExecutionError is intentionally large; boxing it would add complexity
// without meaningful benefit here.
#[allow(clippy::result_large_err)]
pub(super) fn api_error_response(
    status: StatusCode,
    message: impl Into<String>,
) -> Result<Response<AxumBody>, ExecutionError> {
    let mut response = response(
        status,
        serde_json::json!({ "error": message.into() }).to_string(),
    )?;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(response)
}

pub(super) fn http_error_response(
    error: &HttpError,
) -> Result<Response<AxumBody>, ExecutionError> {
    let (status, message) = map_http_error_for_response(error);
    let status = StatusCode::from_u16(status)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    api_error_response(status, message)
}

pub(super) fn method_not_allowed_response(
    allow: &'static str,
) -> Result<Response<AxumBody>, ExecutionError> {
    let mut response = api_error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "Method not allowed",
    )?;
    response
        .headers_mut()
        .insert(ALLOW, HeaderValue::from_static(allow));
    Ok(response)
}

pub(super) fn request_parse_error_response(
    error: RequestParseError,
) -> Result<Response<AxumBody>, ExecutionError> {
    let (status, message, category) = match error {
        RequestParseError::InvalidPath => (
            StatusCode::NOT_FOUND,
            "Invalid URL path".to_string(),
            "invalid_path",
        ),
        RequestParseError::InvalidPayload(msg) => {
            (StatusCode::UNPROCESSABLE_ENTITY, msg, "invalid_payload")
        }
        RequestParseError::Other(err) => {
            if let Some(http_err) = err.downcast_ref::<HttpError>() {
                let (status, message) = map_http_error_for_response(http_err);
                let status = StatusCode::from_u16(status)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                return api_error_response(status, message);
            }
            error!(error = %err, "Failed parsing RUES dispatch request");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed parsing request".to_string(),
                "internal",
            )
        }
    };
    debug!(
        status = %status,
        error_category = category,
        "RUES request parse failed"
    );

    api_error_response(status, message)
}
