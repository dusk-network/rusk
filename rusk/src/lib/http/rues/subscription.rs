// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(not(any(feature = "chain", test)), allow(dead_code))]

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as AxumResponse};
use tokio::sync::{RwLock, mpsc, oneshot};

use super::event::{RuesEventUri, SessionId};
use super::request::event_uri;
use crate::http::error::ApiError;

pub(crate) type SocketMap =
    Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>;

pub(crate) enum SubscriptionAction {
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
pub(crate) enum SubscriptionError {
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

pub(crate) async fn subscribe(
    component: &str,
    entity: Option<&str>,
    topic: &str,
    sid: SessionId,
    sockets_map: SocketMap,
) -> Result<AxumResponse, ApiError> {
    let context = parse_subscription_context_with_uri(
        sid,
        event_uri(component, entity, topic)?,
        sockets_map,
    )
    .await?;
    dispatch_subscribe(context).await
}

pub(crate) async fn unsubscribe(
    component: &str,
    entity: Option<&str>,
    topic: &str,
    sid: SessionId,
    sockets_map: SocketMap,
) -> Result<AxumResponse, ApiError> {
    let context = parse_subscription_context_with_uri(
        sid,
        event_uri(component, entity, topic)?,
        sockets_map,
    )
    .await?;
    dispatch_unsubscribe(context).await
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
