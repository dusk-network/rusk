// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(
    not(any(feature = "chain", feature = "prover", test)),
    allow(dead_code)
)]

use axum::Router;
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Json};
use serde_json::json;
use tower_http::trace::TraceLayer;

use crate::http::middleware::{
    configured_headers_middleware, request_policy_middleware,
};
use crate::http::{HttpAppState, openapi, routes};

pub(crate) fn build_app(state: HttpAppState) -> Router {
    let enable_docs = state.enable_docs;
    let router = routes::router()
        .fallback(not_found)
        .layer(from_fn_with_state(state.clone(), request_policy_middleware))
        .layer(from_fn_with_state(
            state.clone(),
            configured_headers_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state::<()>(state);

    if enable_docs {
        let (router, api) = router.split_for_parts();
        openapi::with_docs_routes(router, api)
    } else {
        router.into()
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn generated_openapi() -> utoipa::openapi::OpenApi {
    routes::router().into_openapi()
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "Path not found" })),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::convert::Infallible;
    use std::sync::Arc;

    use axum::body::{Body, to_bytes};
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Method, Request, StatusCode};
    use serde_json::Value;
    use tokio::sync::{RwLock, broadcast, mpsc};
    use tower::ServiceExt;

    use super::build_app;
    use crate::http::rues::SubscriptionAction;
    use crate::http::{
        HttpAppState, HttpHandlers, HttpPolicyConfig, HttpRequestPolicy,
        RuesEvent, SessionId,
    };

    fn test_state(enable_docs: bool) -> HttpAppState {
        let (events_tx, _events_rx) = broadcast::channel::<RuesEvent>(1);
        let (shutdown_tx, _shutdown_rx) = broadcast::channel::<Infallible>(1);
        HttpAppState {
            services: HttpHandlers::default(),
            sockets_map: Arc::new(RwLock::new(HashMap::<
                SessionId,
                mpsc::Sender<SubscriptionAction>,
            >::new())),
            events: events_tx,
            shutdown: shutdown_tx,
            ws_event_channel_cap: 1,
            enable_docs,
            policy: Arc::new(HttpRequestPolicy::new(
                HttpPolicyConfig::default(),
            )),
            headers: Arc::new(axum::http::HeaderMap::new()),
        }
    }

    async fn insert_test_session(
        state: &HttpAppState,
    ) -> (SessionId, mpsc::Receiver<SubscriptionAction>) {
        let session_id = SessionId::parse("00112233445566778899aabbccddeeff")
            .expect("Session ID should parse");
        let (sender, receiver) = mpsc::channel(1);
        state.sockets_map.write().await.insert(session_id, sender);
        (session_id, receiver)
    }

    async fn assert_entity_subscription_route(
        method: Method,
        path: &str,
        expected_component: &str,
        expected_entity: &str,
        expected_topic: &str,
    ) {
        let state = test_state(false);
        let (session_id, mut receiver) = insert_test_session(&state).await;
        let request = Request::builder()
            .method(method.clone())
            .uri(path)
            .header("Rusk-Session-Id", session_id.to_string())
            .body(Body::empty())
            .expect("Request should be built");
        let app = build_app(state);

        let response_handle = tokio::spawn(async move {
            app.oneshot(request)
                .await
                .expect("Subscription response should be produced")
        });

        let action = receiver
            .recv()
            .await
            .expect("Subscription action should be delivered");

        match action {
            SubscriptionAction::Subscribe { uri, reply }
                if method == Method::GET =>
            {
                assert_eq!(uri.component, expected_component);
                assert_eq!(uri.entity.as_deref(), Some(expected_entity));
                assert_eq!(uri.topic, expected_topic);
                reply.send(Ok(())).expect("Reply should be delivered");
            }
            SubscriptionAction::Unsubscribe { uri, reply }
                if method == Method::DELETE =>
            {
                assert_eq!(uri.component, expected_component);
                assert_eq!(uri.entity.as_deref(), Some(expected_entity));
                assert_eq!(uri.topic, expected_topic);
                reply.send(Ok(())).expect("Reply should be delivered");
            }
            _ => panic!("Unexpected subscription action"),
        }

        let response = response_handle
            .await
            .expect("Subscription task should complete");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn router_fallback_returns_json_not_found() {
        let app = build_app(test_state(false));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/unsupported")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("Fallback response should be produced");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(content_type, "application/json");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Body should be readable");
        let payload: Value =
            serde_json::from_slice(&body).expect("Body should be JSON");
        assert_eq!(payload["error"], "Path not found");
    }

    #[tokio::test]
    async fn docs_routes_are_available_when_enabled() {
        let app = build_app(test_state(true));

        let openapi_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("OpenAPI route should respond");

        assert_eq!(openapi_response.status(), StatusCode::OK);
        let openapi_body = to_bytes(openapi_response.into_body(), usize::MAX)
            .await
            .expect("OpenAPI body should be readable");
        let openapi_json: Value = serde_json::from_slice(&openapi_body)
            .expect("OpenAPI body should be JSON");
        let paths = openapi_json
            .get("paths")
            .and_then(Value::as_object)
            .expect("OpenAPI paths should be present");
        assert!(
            paths.contains_key("/graphql"),
            "OpenAPI document should include /graphql"
        );

        let swagger_response = app
            .oneshot(
                Request::builder()
                    .uri("/swagger-ui/index.html")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("Swagger route should respond");

        assert_eq!(swagger_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn docs_routes_are_not_available_when_disabled() {
        let app = build_app(test_state(false));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("Response should be produced");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn transaction_entity_subscription_routes_preserve_entity_topics() {
        assert_entity_subscription_route(
            Method::GET,
            "/on/transactions:coffee/executed",
            "transactions",
            "coffee",
            "executed",
        )
        .await;

        assert_entity_subscription_route(
            Method::DELETE,
            "/on/transactions:coffee/executed",
            "transactions",
            "coffee",
            "executed",
        )
        .await;
    }

    #[tokio::test]
    async fn block_entity_subscription_routes_preserve_entity_topics() {
        assert_entity_subscription_route(
            Method::GET,
            "/on/blocks:cafe/statechange",
            "blocks",
            "cafe",
            "statechange",
        )
        .await;

        assert_entity_subscription_route(
            Method::DELETE,
            "/on/blocks:cafe/statechange",
            "blocks",
            "cafe",
            "statechange",
        )
        .await;
    }
}
