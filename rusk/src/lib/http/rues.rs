// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(not(any(feature = "chain", test)), allow(dead_code))]

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::extract::ws::{
    CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code,
};
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response as AxumResponse};
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, warn};

use super::axum_app::HttpAppState;
use super::error::{
    ApiError, http_error_category, map_http_error_for_response,
};
use super::event::{SessionId, check_rusk_version};
use super::{
    DataType, EventResponse, ExecutionError, HttpError, RUSK_VERSION_HEADER,
    RUSK_VERSION_STRICT_HEADER, ResponseData, RuesDispatchEvent, RuesEvent,
    RuesEventUri,
};
use crate::VERSION;

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

fn invalid_session_id_error() -> ApiError {
    // TODO: Keep 424 for current RUES compatibility; revisit whether malformed
    // or missing session identifiers should be normalized to 400.
    ApiError::new(
        StatusCode::FAILED_DEPENDENCY,
        "Session ID not provided or invalid",
        "invalid_session",
    )
}

fn failed_consuming_request_error() -> ApiError {
    ApiError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed consuming request",
        "internal",
    )
}

pub(super) fn validate_rusk_version_headers(
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

type SocketMap =
    Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>;

async fn handle_stream_rues(
    sid: SessionId,
    mut stream: WebSocket,
    events: broadcast::Receiver<RuesEvent>,
    mut subscriptions: mpsc::Receiver<SubscriptionAction>,
    mut shutdown: broadcast::Receiver<Infallible>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
) {
    if stream
        .send(Message::Text(sid.to_string().into()))
        .await
        .is_err()
    {
        let _ = stream
            .send(Message::Close(Some(close_frame(
                close_code::ERROR,
                "Failed sending session ID",
            ))))
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
                        let _ = stream.send(Message::Close(msg)).await;
                        break;
                    }
                    Some(Err(e)) => {
                        let _ = stream.send(Message::Close(Some(close_frame(
                            close_code::ERROR,
                            "Internal error",
                        )))).await;
                        warn!("Closing stream for {sid} due to {e}");
                        break;
                    }
                    None => {
                        let _ = stream.send(Message::Close(Some(close_frame(
                            close_code::ERROR,
                            "No more events",
                        )))).await;
                        warn!("Closing stream for {sid} due to no more events");
                        break;
                    }
                    _ => {}
                }
            }
            _ = shutdown.recv() => {
                let _ = stream.send(Message::Close(Some(close_frame(
                    close_code::AWAY,
                    "Shutting down",
                )))).await;
                break;
            }
            subscription = subscriptions.recv() => {
                let subscription = match subscription {
                    Some(subscription) => subscription,
                    None => {
                        // If the subscription channel is closed, it means the server has stopped
                        // communicating with this loop, so we should inform the client and stop.
                        let _ = stream.send(Message::Close(Some(close_frame(
                            close_code::AWAY,
                            "Shutting down",
                        )))).await;
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
                        let _ = stream.send(Message::Close(Some(close_frame(
                            close_code::AWAY,
                            "Shutting down",
                        )))).await;
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
                    if stream.send(Message::Binary(event.into())).await.is_err() {
                        let _ = stream.send(Message::Close(Some(close_frame(
                            close_code::ERROR,
                            "Failed sending event",
                        )))).await;
                        break;
                    }
                }
            }
        }
    }

    let mut sockets = sockets_map.write().await;
    sockets.remove(&sid);
}

fn close_frame(code: u16, reason: &'static str) -> CloseFrame {
    CloseFrame {
        code,
        reason: reason.into(),
    }
}

pub(super) async fn handle_rues_ws(
    State(state): State<HttpAppState>,
    ws: WebSocketUpgrade,
) -> AxumResponse {
    let events = state.events.subscribe();
    let shutdown = state.shutdown.subscribe();
    let (subscription_sender, subscriptions) =
        mpsc::channel::<SubscriptionAction>(state.ws_event_channel_cap);

    let mut sockets = state.sockets_map.write().await;

    // This is a new WebSocket connection, so we generate a new random ID
    // and create a new channel for it.
    let mut sid = rand::random();
    while sockets.contains_key(&sid) {
        sid = rand::random();
    }
    sockets.insert(sid, subscription_sender);
    drop(sockets);

    ws.max_message_size(super::MAX_WS_INBOUND_MESSAGE_BYTES)
        .max_frame_size(super::MAX_WS_INBOUND_FRAME_BYTES)
        .on_upgrade(move |socket| {
            handle_stream_rues(
                sid,
                socket,
                events,
                subscriptions,
                shutdown,
                state.sockets_map,
            )
        })
}

pub(super) async fn dispatch_rues_subscribe(
    sid: SessionId,
    sockets_map: SocketMap,
    uri: RuesEventUri,
) -> Result<AxumResponse, ApiError> {
    let context =
        parse_subscription_context_with_uri(sid, uri, sockets_map).await?;
    dispatch_subscribe(context).await
}

pub(super) async fn dispatch_rues_unsubscribe(
    sid: SessionId,
    sockets_map: SocketMap,
    uri: RuesEventUri,
) -> Result<AxumResponse, ApiError> {
    let context =
        parse_subscription_context_with_uri(sid, uri, sockets_map).await?;
    dispatch_unsubscribe(context).await
}

pub(super) fn parse_rues_post(
    uri: RuesEventUri,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(RuesDispatchEvent, bool), ApiError> {
    RuesDispatchEvent::from_uri_headers_and_body(uri, &headers, body.to_vec())
        .map_err(ApiError::from)
}

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

struct SubscriptionRequestContext {
    uri: RuesEventUri,
    action_sender: mpsc::Sender<SubscriptionAction>,
}

async fn parse_subscription_context_with_uri(
    sid: SessionId,
    uri: RuesEventUri,
    sockets_map: SocketMap,
) -> Result<SubscriptionRequestContext, ApiError> {
    let action_sender: mpsc::Sender<SubscriptionAction> =
        match sockets_map.read().await.get(&sid) {
            Some(sender) => sender.clone(),
            None => return Err(invalid_session_id_error()),
        };

    Ok(SubscriptionRequestContext { uri, action_sender })
}

async fn dispatch_subscribe(
    context: SubscriptionRequestContext,
) -> Result<AxumResponse, ApiError> {
    let (reply, receiver) = oneshot::channel();
    let action = SubscriptionAction::Subscribe {
        uri: context.uri,
        reply,
    };

    if context.action_sender.send(action).await.is_err() {
        return Err(failed_consuming_request_error());
    }

    match receiver.await {
        Ok(Ok(())) => Ok(StatusCode::OK.into_response()),
        Ok(Err(SubscriptionError::NotFound)) => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "Subscription not found",
            "not_found",
        )),
        Err(_) => Err(failed_consuming_request_error()),
    }
}

async fn dispatch_unsubscribe(
    context: SubscriptionRequestContext,
) -> Result<AxumResponse, ApiError> {
    let (reply, receiver) = oneshot::channel();
    let action = SubscriptionAction::Unsubscribe {
        uri: context.uri,
        reply,
    };

    if context.action_sender.send(action).await.is_err() {
        return Err(failed_consuming_request_error());
    }

    match receiver.await {
        Ok(Ok(())) => Ok(StatusCode::OK.into_response()),
        Ok(Err(SubscriptionError::NotFound)) => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "Subscription not found",
            "not_found",
        )),
        Err(_) => Err(failed_consuming_request_error()),
    }
}

fn handle_execution_rues(
    event: &RuesDispatchEvent,
    result: Result<ResponseData, HttpError>,
) -> EventResponse {
    let mut rsp = result
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
    rsp
}
