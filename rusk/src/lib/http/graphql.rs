// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_graphql::http::{
    MultipartOptions, parse_query_string, receive_batch_body,
};
use async_graphql::{
    BatchRequest, BatchResponse, ParseRequestError,
    Response as GraphqlResponse, ServerError,
};
use futures_util::io::Cursor;
use http_body_util::{BodyExt, LengthLimitError, Limited};
use hyper::header::{ALLOW, CONTENT_TYPE};
use hyper::http::{HeaderName, HeaderValue};
use hyper::{Method, Request, Response, StatusCode, body::Incoming};

use crate::http::event::FullOrStreamBody;

use super::{
    ExecutionError, GraphqlHandler, MAX_GRAPHQL_REQUEST_BODY_BYTES, response,
};

pub(super) fn is_graphql_path(path: &str) -> bool {
    matches!(path, "/graphql" | "/graphql/")
}

// ExecutionError is intentionally large; boxing it would add complexity
// without meaningful benefit here.
#[allow(clippy::result_large_err)]
fn graphql_batch_response(
    status: StatusCode,
    batch_response: BatchResponse,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    let body = serde_json::to_vec(&batch_response)?;
    let mut response = response(status, body)?;
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    for (name, value) in batch_response.http_headers_iter() {
        let name = HeaderName::from_bytes(name.as_str().as_bytes())?;
        let value = HeaderValue::from_bytes(value.as_bytes())?;
        headers.append(name, value);
    }
    Ok(response)
}

// ExecutionError is intentionally large; boxing it would add complexity
// without meaningful benefit here.
#[allow(clippy::result_large_err)]
fn graphql_error_response(
    status: StatusCode,
    message: impl Into<String>,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    let error = ServerError::new(message, None);
    let response = GraphqlResponse::from_errors(vec![error]);
    graphql_batch_response(status, BatchResponse::from(response))
}

// ExecutionError is intentionally large; boxing it would add complexity
// without meaningful benefit here.
#[allow(clippy::result_large_err)]
pub(super) fn handle_graphql_http_error(
    status: StatusCode,
    message: impl Into<String>,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    graphql_error_response(status, message)
}

fn graphql_parse_error_status(error: &ParseRequestError) -> StatusCode {
    match error {
        ParseRequestError::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        ParseRequestError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::BAD_REQUEST,
    }
}

pub(super) async fn handle_graphql_http(
    handler: &dyn GraphqlHandler,
    req: Request<Incoming>,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    match *req.method() {
        Method::GET => {
            let query = req.uri().query().unwrap_or_default();
            if query.is_empty() {
                return graphql_error_response(
                    StatusCode::BAD_REQUEST,
                    "GraphQL GET requests require a query parameter",
                );
            }

            let request = match parse_query_string(query) {
                Ok(request) => request,
                Err(err) => {
                    return graphql_error_response(
                        graphql_parse_error_status(&err),
                        err.to_string(),
                    );
                }
            };

            let batch_response =
                handler.execute_graphql(BatchRequest::Single(request)).await;
            graphql_batch_response(StatusCode::OK, batch_response)
        }
        Method::POST => {
            let (parts, body) = req.into_parts();
            let content_type = parts
                .headers
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok());
            let body = match Limited::new(body, MAX_GRAPHQL_REQUEST_BODY_BYTES)
                .collect()
                .await
            {
                Ok(collected) => collected.to_bytes().to_vec(),
                Err(err) => {
                    if err.downcast_ref::<LengthLimitError>().is_some() {
                        return graphql_error_response(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            format!(
                                "Request body exceeds {MAX_GRAPHQL_REQUEST_BODY_BYTES} bytes"
                            ),
                        );
                    }

                    return graphql_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        err.to_string(),
                    );
                }
            };
            let reader = Cursor::new(body);

            let batch_request = receive_batch_body(
                content_type,
                reader,
                MultipartOptions::default(),
            )
            .await;

            match batch_request {
                Ok(batch_request) => {
                    let batch_response =
                        handler.execute_graphql(batch_request).await;
                    graphql_batch_response(StatusCode::OK, batch_response)
                }
                Err(err) => graphql_error_response(
                    graphql_parse_error_status(&err),
                    err.to_string(),
                ),
            }
        }
        _ => {
            let mut response = graphql_error_response(
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed",
            )?;
            response
                .headers_mut()
                .insert(ALLOW, HeaderValue::from_static("GET, POST"));
            Ok(response)
        }
    }
}
