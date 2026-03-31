// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(not(feature = "chain"), allow(dead_code))]

use axum::body::Bytes;
use axum::http::HeaderMap;
use axum::response::Response as AxumResponse;

use super::event::{
    ResponseData, RuesDispatchEvent, RuesEventUri, check_rusk_version,
};
use super::response::finish_rues_post;
use crate::http::error::ApiError;
use crate::http::{HttpError, RUSK_VERSION_HEADER, RUSK_VERSION_STRICT_HEADER};

pub(super) fn event_uri(
    component: &str,
    entity: Option<&str>,
    topic: &str,
) -> Result<RuesEventUri, ApiError> {
    RuesEventUri::from_parts(component, entity.map(ToOwned::to_owned), topic)
        .ok_or_else(invalid_rues_path_error)
}

pub(crate) fn validate_rusk_version_headers(
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    let strict = headers.contains_key(RUSK_VERSION_STRICT_HEADER);
    let version = match headers.get(RUSK_VERSION_HEADER) {
        Some(value) => {
            let value_str = value.to_str().map_err(|_| {
                HttpError::VersionMismatch(
                    "Invalid Rusk-Version header encoding".to_string(),
                )
            })?;
            Some(serde_json::Value::String(value_str.to_owned()))
        }
        None => None,
    };

    check_rusk_version(version.as_ref(), strict)?;
    Ok(())
}

fn invalid_rues_path_error() -> ApiError {
    ApiError::new(
        axum::http::StatusCode::NOT_FOUND,
        "Invalid URL path",
        "invalid_path",
    )
}

pub(crate) struct ParsedRuesRequest {
    event: RuesDispatchEvent,
    binary_request: bool,
}

impl ParsedRuesRequest {
    pub(crate) fn component(
        component: &str,
        topic: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<Self, ApiError> {
        Self::new(event_uri(component, None, topic)?, headers, body)
    }

    pub(crate) fn entity(
        component: &str,
        entity: &str,
        topic: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<Self, ApiError> {
        Self::new(event_uri(component, Some(entity), topic)?, headers, body)
    }

    fn new(
        uri: RuesEventUri,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<Self, ApiError> {
        let (event, binary_request) =
            RuesDispatchEvent::from_uri_headers_and_body(
                uri,
                &headers,
                body.to_vec(),
            )
            .map_err(ApiError::from)?;
        Ok(Self {
            event,
            binary_request,
        })
    }

    pub(crate) fn event(&self) -> &RuesDispatchEvent {
        &self.event
    }

    pub(crate) fn into_response(
        self,
        result: Result<ResponseData, HttpError>,
    ) -> Result<AxumResponse, ApiError> {
        finish_rues_post(self.event, self.binary_request, result)
    }
}
