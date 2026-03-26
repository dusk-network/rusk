// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::str::FromStr;

use axum::http::StatusCode;
use axum::http::header::{HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response as AxumResponse};
use tracing::{debug, error};

use super::event::{
    DataType, ExecutionError, MessageResponse, ResponseData, RuesDispatchEvent,
};
use crate::VERSION;
use crate::http::error::{
    ApiError, http_error_category, map_http_error_for_response,
};
use crate::http::{HttpError, RUSK_VERSION_HEADER};

pub(super) fn finish_rues_post(
    event: RuesDispatchEvent,
    binary_request: bool,
    result: Result<ResponseData, HttpError>,
) -> Result<AxumResponse, ApiError> {
    let mut resp_headers = event.x_headers();
    let mut execution_response = handle_execution_rues(&event, result);
    resp_headers.extend(execution_response.headers.clone());
    execution_response.force_binary |= binary_request;
    let is_empty = execution_response.error.is_none()
        && matches!(execution_response.data, DataType::None);
    let mut resp = execution_response.into_response();
    if is_empty {
        *resp.status_mut() = StatusCode::ACCEPTED;
    }

    for (k, v) in resp_headers {
        let k = HeaderName::from_str(&k)
            .map_err(ExecutionError::from)
            .map_err(ApiError::from)?;
        let v = match v {
            serde_json::Value::String(s) => HeaderValue::from_str(&s),
            serde_json::Value::Null => HeaderValue::from_str(""),
            _ => HeaderValue::from_str(&v.to_string()),
        }
        .map_err(ExecutionError::from)
        .map_err(ApiError::from)?;
        resp.headers_mut().append(k, v);
    }

    Ok(resp)
}

fn handle_execution_rues(
    event: &RuesDispatchEvent,
    result: Result<ResponseData, HttpError>,
) -> MessageResponse {
    let mut rsp = result
        .map(|data| {
            let (data, mut headers, force_binary) = data.into_inner();
            headers.append(&mut event.x_headers());
            MessageResponse {
                data,
                error: None,
                headers,
                force_binary,
            }
        })
        .unwrap_or_else(|e| {
            let (status, message) = map_http_error_for_response(&e);
            let category = http_error_category(&e);
            if status >= 500 {
                error!(
                    status,
                    error_category = category,
                    error = %e,
                    "RUES handler failed"
                );
            } else {
                debug!(
                    status,
                    error_category = category,
                    error = %e,
                    "RUES handler rejected request"
                );
            }
            MessageResponse {
                headers: event.x_headers(),
                data: DataType::None,
                error: Some((message, status)),
                force_binary: false,
            }
        });

    rsp.set_header(RUSK_VERSION_HEADER, serde_json::json!(*VERSION));
    rsp
}
