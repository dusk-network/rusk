// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashSet;
use std::convert::Infallible;

use axum::extract::State;
use axum::extract::ws::{
    CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code,
};
use axum::response::Response as AxumResponse;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, warn};

use super::event::{RuesEvent, SessionId};
use super::subscription::{SocketMap, SubscriptionAction, SubscriptionError};
use crate::http::HttpAppState;

async fn handle_stream_rues(
    sid: SessionId,
    mut stream: WebSocket,
    events: broadcast::Receiver<RuesEvent>,
    mut subscriptions: mpsc::Receiver<SubscriptionAction>,
    mut shutdown: broadcast::Receiver<Infallible>,
    sockets_map: SocketMap,
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

/// Upgrade endpoint for the legacy RUES WebSocket transport.
///
/// This route documents only the HTTP handshake. Swagger UI is not expected to
/// drive the socket interactively.
///
/// After a successful `101` upgrade, the server sends a first text frame which
/// contains the generated session identifier. HTTP subscription routes under
/// `/on/*` expect that identifier to be echoed back via the `Rusk-Session-Id`
/// header.
#[utoipa::path(
    get,
    path = "/on",
    tag = "RUES",
    responses(
        (status = 101, description = "WebSocket upgrade successful, session ID follows"),
        (status = 400, description = "Invalid WebSocket upgrade request"),
    )
)]
pub(crate) async fn handle_rues_ws(
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

    ws.max_message_size(super::super::MAX_WS_INBOUND_MESSAGE_BYTES)
        .max_frame_size(super::super::MAX_WS_INBOUND_FRAME_BYTES)
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
