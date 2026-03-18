// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::str::FromStr;
use std::sync::Arc;

use futures_util::SinkExt;
use hyper::body::Incoming;
use hyper::http::{HeaderName, HeaderValue};
use hyper::{HeaderMap, Method, Request, Response, StatusCode};
use hyper_tungstenite::{HyperWebsocket, tungstenite};
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio::task;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, warn};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Message};

use super::error::{http_error_category, map_http_error_for_response};
use super::event::check_rusk_version;
use super::responses::{
    api_error_response, http_error_response, method_not_allowed_response,
    request_parse_error_response,
};
use super::{
    DataType, EventResponse, ExecutionError, HandleRequest, HttpError,
    RUES_LOCATION_PREFIX, RUSK_VERSION_HEADER, RUSK_VERSION_STRICT_HEADER,
    RuesDispatchEvent, RuesEvent, RuesEventUri, SessionId, response,
};
use crate::VERSION;
use crate::http::event::FullOrStreamBody;

pub(super) enum SubscriptionAction {
    Subscribe {
        uri: RuesEventUri,
        reply: oneshot::Sender<Result<(), SubscriptionError>>,
    },
    Unsubscribe {
        uri: RuesEventUri,
        reply: oneshot::Sender<Result<(), SubscriptionError>>,
    },
}

#[derive(Debug)]
pub(super) enum SubscriptionError {
    NotFound,
}

fn invalid_rues_path_response()
-> Result<Response<FullOrStreamBody>, ExecutionError> {
    api_error_response(StatusCode::NOT_FOUND, "Invalid URL path")
}

fn invalid_session_id_response()
-> Result<Response<FullOrStreamBody>, ExecutionError> {
    // TODO: Keep 424 for current RUES compatibility; revisit whether malformed
    // or missing session identifiers should be normalized to 400.
    api_error_response(
        StatusCode::FAILED_DEPENDENCY,
        "Session ID not provided or invalid",
    )
}

fn failed_consuming_request_response()
-> Result<Response<FullOrStreamBody>, ExecutionError> {
    api_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed consuming request",
    )
}

async fn handle_stream_rues(
    sid: SessionId,
    websocket: HyperWebsocket,
    events: broadcast::Receiver<RuesEvent>,
    mut subscriptions: mpsc::Receiver<SubscriptionAction>,
    mut shutdown: broadcast::Receiver<Infallible>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
) {
    let mut stream = match websocket.await {
        Ok(stream) => stream,
        Err(_) => return,
    };

    if stream.send(Message::Text(sid.to_string())).await.is_err() {
        let _ = stream
            .close(Some(CloseFrame {
                code: CloseCode::Error,
                reason: Cow::from("Failed sending session ID"),
            }))
            .await;
        return;
    }

    let mut subscription_set = HashSet::new();
    let mut events = BroadcastStream::new(events);

    loop {
        tokio::select! {
            recv = stream.next() => {
                match recv {
                    Some(Ok(Message::Close(msg))) => {
                        debug!("Closing stream for {sid} due to {msg:?}");
                        let _ = stream.close(msg).await;
                        break;
                    }
                    Some(Err(e)) => {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from("Internal error"),
                        })).await;
                        warn!("Closing stream for {sid} due to {e}");
                        break;
                    }
                    None => {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from("No more events"),
                        })).await;
                        warn!("Closing stream for {sid} due to no more events");
                        break;
                    }
                    _ => {}
                }
            }
            _ = shutdown.recv() => {
                let _ = stream.close(Some(CloseFrame {
                    code: CloseCode::Away,
                    reason: Cow::from("Shutting down"),
                })).await;
                break;
            }
            subscription = subscriptions.recv() => {
                let subscription = match subscription {
                    Some(subscription) => subscription,
                    None => {
                        // If the subscription channel is closed, it means the server has stopped
                        // communicating with this loop, so we should inform the client and stop.
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Away,
                            reason: Cow::from("Shutting down"),
                        })).await;
                        break;
                    },
                };

                match subscription {
                    SubscriptionAction::Subscribe { uri, reply } => {
                        subscription_set.insert(uri);
                        let _ = reply.send(Ok(()));
                    },
                    SubscriptionAction::Unsubscribe { uri, reply } => {
                        if subscription_set.remove(&uri) {
                            let _ = reply.send(Ok(()));
                        } else {
                            let _ = reply.send(Err(SubscriptionError::NotFound));
                        }
                    },
                }
            }
            Some(event) = events.next() => {
                let mut event = match event {
                    Ok(event) => event,
                    Err(_) => {
                        // If the event channel is closed, it means the
                        // server has stopped producing events, so we
                        // should inform the client and stop.
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Away,
                            reason: Cow::from("Shutting down"),
                        })).await;
                        break;

                    }
                };

                // The event is subscribed to if it matches any of the subscriptions.
                let mut is_subscribed = false;
                for sub in &subscription_set {
                    if sub.matches(&event) {
                        is_subscribed = true;
                        break;
                    }
                }

                // If the event is subscribed, we send it to the client.
                if is_subscribed {
                    event.add_header("Content-Location", event.uri.to_string());
                    let event = event.to_bytes();

                    // If the event fails sending we close the socket on the client
                    // and stop processing further.
                    if stream.send(Message::Binary(event)).await.is_err() {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from("Failed sending event"),
                        })).await;
                        break;
                    }
                }
            }
        }
    }

    let mut sockets = sockets_map.write().await;
    sockets.remove(&sid);
}

pub(super) async fn handle_request_rues<H: HandleRequest>(
    mut req: Request<Incoming>,
    handler: Arc<H>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
    events: broadcast::Receiver<RuesEvent>,
    shutdown: broadcast::Receiver<Infallible>,
    ws_event_channel_cap: usize,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    if hyper_tungstenite::is_upgrade_request(&req) {
        let (subscription_sender, subscriptions) =
            mpsc::channel(ws_event_channel_cap);

        let ws_config = tungstenite::protocol::WebSocketConfig {
            max_message_size: Some(super::MAX_WS_INBOUND_MESSAGE_BYTES),
            max_frame_size: Some(super::MAX_WS_INBOUND_FRAME_BYTES),
            ..Default::default()
        };
        let (response, websocket) =
            hyper_tungstenite::upgrade(&mut req, Some(ws_config))?;

        let mut sockets = sockets_map.write().await;

        // This is a new WebSocket connection, so we generate a new random ID
        // and create a new channel for it.
        let mut sid = rand::random();
        while sockets.contains_key(&sid) {
            sid = rand::random();
        }
        sockets.insert(sid, subscription_sender);

        task::spawn(handle_stream_rues(
            sid,
            websocket,
            events,
            subscriptions,
            shutdown,
            sockets_map.clone(),
        ));

        Ok(response.map(Into::into))
    } else if req.method() == Method::POST {
        if let Err(err) = validate_rusk_version_headers(req.headers()) {
            return http_error_response(&err);
        }

        let (event, binary_request) =
            match RuesDispatchEvent::from_request(req).await {
                Ok(event) => event,
                Err(err) => return request_parse_error_response(err),
            };
        let mut resp_headers = event.x_headers();
        let (responder, mut receiver) = mpsc::unbounded_channel();
        handle_execution_rues(handler, event, responder).await;

        let execution_response = match receiver.recv().await {
            Some(response) => response,
            None => {
                error!("RUES execution response channel closed unexpectedly");
                return api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error",
                );
            }
        };
        resp_headers.extend(execution_response.headers.clone());
        let binary_response = binary_request || execution_response.force_binary;
        let is_empty = execution_response.error.is_none()
            && matches!(execution_response.data, DataType::None);
        let mut resp = execution_response.into_http(binary_response)?;
        if is_empty {
            *resp.status_mut() = StatusCode::ACCEPTED;
        }

        for (k, v) in resp_headers {
            let k = HeaderName::from_str(&k)?;
            let v = match v {
                serde_json::Value::String(s) => HeaderValue::from_str(&s),
                serde_json::Value::Null => HeaderValue::from_str(""),
                _ => HeaderValue::from_str(&v.to_string()),
            }?;
            resp.headers_mut().append(k, v);
        }

        Ok(resp)
    } else {
        if let Err(err) = validate_rusk_version_headers(req.headers()) {
            return http_error_response(&err);
        }

        let sid = match SessionId::parse_from_req(&req) {
            None => return invalid_session_id_response(),
            Some(sid) => sid,
        };

        let uri = match RuesEventUri::parse_from_path(req.uri().path()) {
            None => return invalid_rues_path_response(),
            Some(s) => s,
        };

        let action_sender = match sockets_map.read().await.get(&sid) {
            Some(sender) => sender.clone(),
            None => return invalid_session_id_response(),
        };

        let (action, reply) = match *req.method() {
            Method::GET => {
                let (reply, receiver) = oneshot::channel();
                (SubscriptionAction::Subscribe { uri, reply }, receiver)
            }
            Method::DELETE => {
                let (reply, receiver) = oneshot::channel();
                (SubscriptionAction::Unsubscribe { uri, reply }, receiver)
            }
            _ => return method_not_allowed_response("GET, DELETE"),
        };

        if action_sender.send(action).await.is_err() {
            return failed_consuming_request_response();
        }

        match reply.await {
            Ok(Ok(())) => response(StatusCode::OK, ""),
            Ok(Err(SubscriptionError::NotFound)) => api_error_response(
                StatusCode::NOT_FOUND,
                "Subscription not found",
            ),
            // TODO: consider returning 424 instead of 500 for reply channel
            // closure during session teardown
            Err(_) => failed_consuming_request_response(),
        }
    }
}

fn validate_rusk_version_headers(headers: &HeaderMap) -> Result<(), HttpError> {
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
    check_rusk_version(version.as_ref(), strict)
}

async fn handle_execution_rues<H>(
    sources: Arc<H>,
    event: RuesDispatchEvent,
    responder: mpsc::UnboundedSender<EventResponse>,
) where
    H: HandleRequest,
{
    let mut rsp = sources
        .handle_rues(&event)
        .await
        .map(|data| {
            let (data, mut headers, force_binary) = data.into_inner();
            headers.append(&mut event.x_headers());
            EventResponse {
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
            EventResponse {
                headers: event.x_headers(),
                data: DataType::None,
                error: Some((message, status)),
                force_binary: false,
            }
        });

    rsp.set_header(RUSK_VERSION_HEADER, serde_json::json!(*VERSION));
    let _ = responder.send(rsp);
}

pub(super) fn is_rues_path(path: &str) -> bool {
    path.starts_with(RUES_LOCATION_PREFIX)
}
