use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Response};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::http::error::ApiError;
use crate::http::routes::on::EntityTopicPath;
use crate::http::{
    HttpAppState, HttpError, MAX_DRIVER_UPLOAD_BODY_BYTES,
    MAX_RUES_REQUEST_BODY_BYTES, SessionId, rues,
};

pub(crate) fn contract_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .routes(
            routes!(contracts_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(routes!(contracts_entity_subscribe))
        .routes(routes!(contracts_entity_unsubscribe))
        .routes(
            routes!(driver_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(
            routes!(contract_owner_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(
            routes!(contract_upload_driver_post).layer(
                RequestBodyLimitLayer::new(MAX_DRIVER_UPLOAD_BODY_BYTES),
            ),
        )
        .routes(
            routes!(contract_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
}

/// Smart contract method invocations.
///
/// `POST` calls contract methods with parameters such as `leaves_from_pos`,
/// `existing_nullifiers`, `account`, `opening`, `get_stake`,
/// `pending_withdrawals`, and `chain_id`.
#[utoipa::path(
    post,
    path = "/contracts:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::FeederHeader,
        ("entity" = String, Path, description = "Smart contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract function or query topic")
    ),
    request_body(
        content(
            (String = "application/octet-stream"),
            (serde_json::Value = "application/json")
        ),
        description = "Binary contract arguments or JSON that will be encoded by the contract data driver."
    ),
    responses(
        (status = 200, description = "Contract query response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid contract query, feeder usage, or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 404, description = "Contract or contract data driver not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while executing the contract query", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Contracts handler not configured", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn contracts_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::entity(
        "contracts",
        &entity,
        &topic,
        headers,
        body,
    )?;
    let result = match state.services.rusk_handler() {
        Some(rusk) => rusk.contracts(&entity, &topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Subscribe to contract events and state changes.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake for event delivery.
#[utoipa::path(
    get,
    path = "/contracts:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("entity" = String, Path, description = "Smart contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract event type to monitor")
    ),
    responses(
        (status = 200, description = "Contract event subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn contracts_entity_subscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::subscribe(
        "contracts",
        Some(&entity),
        &topic,
        session_id,
        state.sockets_map,
    )
    .await
}

/// Unsubscribe from contract event notifications.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[utoipa::path(
    delete,
    path = "/contracts:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("entity" = String, Path, description = "Smart contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract event subscription to remove")
    ),
    responses(
        (status = 200, description = "Contract event subscription removed"),
        (status = 404, description = "Subscription not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn contracts_entity_unsubscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::unsubscribe(
        "contracts",
        Some(&entity),
        &topic,
        session_id,
        state.sockets_map,
    )
    .await
}

/// Contract WASM driver queries and management.
///
/// Supported topics are `decode_event[:name]`, `decode_input_fn[:name]`,
/// `decode_output_fn[:name]`, `encode_input_fn[:name]`, `get_schema`, and
/// `get_version`.
#[utoipa::path(
    post,
    path = "/driver:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Hex-encoded contract ID"),
        ("topic" = String, Path, description = "Driver topic: decode_event[:name] | decode_input_fn[:name] | decode_output_fn[:name] | encode_input_fn[:name] | get_schema | get_version")
    ),
    request_body(
        content(
            (String = "application/octet-stream"),
            (serde_json::Value = "application/json")
        ),
        description = "Binary payload for decode operations or JSON input for `encode_input_fn`."
    ),
    responses(
        (status = 200, description = "Driver operation response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid driver request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 404, description = "Driver or contract not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while executing the driver operation", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Driver handler not configured", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn driver_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::entity(
        "driver", &entity, &topic, headers, body,
    )?;
    let result = match state.services.rusk_handler() {
        Some(rusk) => rusk.driver(&entity, &topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Contract ownership and access-control queries.
///
/// The current handler ignores `topic` and resolves owner by contract entity.
/// The topic segment is preserved for legacy route compatibility.
#[utoipa::path(
    post,
    path = "/contract_owner:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Hex-encoded contract ID"),
        ("topic" = String, Path, description = "Legacy placeholder (currently ignored)")
    ),
    responses(
        (status = 200, description = "Contract owner bytes or hex-encoded owner", content(
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid contract owner query or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 404, description = "Contract owner not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving contract ownership", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Contract-owner handler not configured", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn contract_owner_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::entity(
        "contract_owner",
        &entity,
        &topic,
        headers,
        body,
    )?;
    let result = match state.services.rusk_handler() {
        Some(rusk) => {
            rusk.contract_owner(&entity, &topic, request.event()).await
        }
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Upload a contract WASM driver.
///
/// The request body contains binary WASM bytecode. Additional headers such as
/// `sign` may be required depending on the contract's upload authorization.
#[utoipa::path(
    post,
    path = "/contract:{entity}/upload_driver",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::UploadDriverHeader,
        ("entity" = String, Path, description = "Target contract address or ID (hex)")
    ),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "WASM driver bytecode."
    ),
    responses(
        (status = 200, description = "Driver uploaded and verified", body = String, content_type = "text/plain"),
        (status = 400, description = "Invalid driver bytecode, signature, or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 404, description = "Contract owner or contract metadata not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the upload-driver size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while storing the uploaded driver", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Contract handler not configured", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn contract_upload_driver_post(
    State(state): State<HttpAppState>,
    Path(entity): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let topic = "upload_driver".to_string();
    let request = rues::ParsedRuesRequest::entity(
        "contract", &entity, &topic, headers, body,
    )?;
    let result = match state.services.rusk_handler() {
        Some(rusk) => rusk.contract(&entity, &topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Direct contract method invocations.
///
/// Supported topics include `status` (contract balance query), `metadata`, and
/// `download_driver`. The dedicated upload endpoint is documented separately at
/// `/on/contract:{entity}/upload_driver`.
#[utoipa::path(
    post,
    path = "/contract:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract topic: status | metadata | download_driver")
    ),
    responses(
        (status = 200, description = "Contract status, metadata, or driver response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid contract request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 404, description = "Contract metadata, driver, or status not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the contract request", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Contract handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn contract_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::entity(
        "contract", &entity, &topic, headers, body,
    )?;
    let result = match topic.as_str() {
        "status" => match state.services.chain_handler() {
            Some(chain) => {
                chain.contract(&entity, &topic, request.event()).await
            }
            None => Err(HttpError::Unsupported),
        },
        _ => match state.services.rusk_handler() {
            Some(rusk) => rusk.contract(&entity, &topic, request.event()).await,
            None => Err(HttpError::Unsupported),
        },
    };
    request.into_response(result)
}
