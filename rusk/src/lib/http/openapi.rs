// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! OpenAPI documentation for the Rusk HTTP API.

use axum::Router;
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "chain", feature = "prover"))]
use utoipa::IntoParams;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::http::HttpAppState;

#[derive(Serialize, ToSchema)]
pub(super) struct ErrorEnvelope {
    error: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub(super) struct GraphqlHttpRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<serde_json::Value>,
    extensions: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub(super) struct GraphqlHttpError {
    message: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub(super) struct GraphqlHttpResponse {
    data: Option<serde_json::Value>,
    errors: Option<Vec<GraphqlHttpError>>,
}

#[derive(Serialize, ToSchema)]
pub(super) struct RuesErrorResponse {
    error: String,
}

#[cfg(any(feature = "chain", feature = "prover"))]
#[derive(IntoParams)]
#[into_params(parameter_in = Header)]
pub(super) struct VersionHeaders {
    #[param(rename = "Rusk-Version", required = false)]
    _rusk_version: Option<String>,
    #[param(rename = "Rusk-Version-Strict", required = false)]
    _rusk_version_strict: Option<String>,
}

#[cfg(feature = "chain")]
#[derive(IntoParams)]
#[into_params(parameter_in = Header)]
pub(super) struct SessionHeader {
    #[param(rename = "Rusk-Session-Id")]
    _rusk_session_id: String,
}

#[cfg(feature = "chain")]
#[derive(IntoParams)]
#[into_params(parameter_in = Header)]
pub(super) struct FeederHeader {
    #[param(rename = "Rusk-Feeder", required = false)]
    _rusk_feeder: Option<String>,
}

#[cfg(feature = "chain")]
#[derive(IntoParams)]
#[into_params(parameter_in = Header)]
pub(super) struct UploadDriverHeader {
    #[param(rename = "sign")]
    _sign: String,
}

#[cfg(feature = "chain")]
#[derive(IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct GraphqlGetParams {
    #[param(rename = "query")]
    _query: String,
    #[param(rename = "operationName", required = false)]
    _operation_name: Option<String>,
    #[param(rename = "variables", required = false)]
    _variables: Option<String>,
    #[param(rename = "extensions", required = false)]
    _extensions: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    components(
        schemas(
            ErrorEnvelope,
            RuesErrorResponse,
            GraphqlHttpRequest,
            GraphqlHttpResponse,
            GraphqlHttpError,
        )
    ),
    tags(
        (
            name = "RUES",
            description = "RUES WebSocket base endpoint."
        ),
        (
            name = "RUES / Dispatch",
            description = "RUES request-response dispatch endpoints under `/on/*`."
        ),
        (
            name = "RUES / Events",
            description = "RUES event subscription and unsubscription endpoints."
        ),
        (name = "GraphQL", description = "GraphQL HTTP endpoints"),
        (name = "Static", description = "Static asset serving"),
    )
)]
struct ApiDoc;

/// Constructs the OpenAPI router for the Rusk HTTP API, which serves the API
/// documentation at /api-docs/openapi.json and the Swagger UI at /swagger-ui.
pub(super) fn router() -> OpenApiRouter<HttpAppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
}

pub(super) fn with_docs_routes(
    router: Router,
    api: utoipa::openapi::OpenApi,
) -> Router {
    router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::Value;

    use super::super::axum_app;

    fn operation<'a>(spec: &'a Value, path: &str, method: &str) -> &'a Value {
        spec.get("paths")
            .and_then(|paths| paths.get(path))
            .and_then(|item| item.get(method))
            .unwrap_or_else(|| {
                panic!("missing {method} operation for documented path {path}")
            })
    }

    fn parameter_names(op: &Value) -> BTreeSet<&str> {
        op.get("parameters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|param| param.get("name").and_then(Value::as_str))
            .collect()
    }

    fn response_statuses(op: &Value) -> BTreeSet<&str> {
        op.get("responses")
            .and_then(Value::as_object)
            .map(|responses| responses.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn operation_tags(op: &Value) -> BTreeSet<&str> {
        op.get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect()
    }

    fn documented_tag_names(spec: &Value) -> BTreeSet<&str> {
        spec.get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|tag| tag.get("name").and_then(Value::as_str))
            .collect()
    }

    fn request_content_types(op: &Value) -> BTreeSet<&str> {
        op.get("requestBody")
            .and_then(|body| body.get("content"))
            .and_then(Value::as_object)
            .map(|content| content.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn response_content_types<'a>(
        op: &'a Value,
        status: &str,
    ) -> BTreeSet<&'a str> {
        op.get("responses")
            .and_then(|responses| responses.get(status))
            .and_then(|response| response.get("content"))
            .and_then(Value::as_object)
            .map(|content| content.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    #[test]
    fn openapi_document_contains_expected_routes_and_schemas() {
        let spec = axum_app::generated_openapi();

        let paths = spec.paths;
        let path_count = paths.paths.len();

        // Verify we have documented at least 18 paths
        assert!(
            path_count >= 20,
            "Expected at least 20 paths in OpenAPI spec, found {path_count}",
        );

        // Verify key paths are present
        let path_names: Vec<&str> =
            paths.paths.iter().map(|(p, _)| p.as_str()).collect();

        assert!(
            path_names.contains(&"/on"),
            "Missing /on WebSocket endpoint"
        );
        assert!(
            path_names.contains(&"/graphql"),
            "Missing /graphql endpoint"
        );
        assert!(
            path_names.contains(&"/on/transactions/{topic}"),
            "Missing transaction subscription routes"
        );
        assert!(
            path_names.contains(&"/on/transactions:{entity}/{topic}"),
            "Missing entity-scoped transaction subscription routes"
        );
        assert!(
            path_names.contains(&"/on/blocks:{entity}/{topic}"),
            "Missing entity-scoped block subscription routes"
        );
        assert!(
            path_names.contains(&"/on/contracts:{entity}/{topic}"),
            "Missing contract interaction routes"
        );
        assert!(
            path_names.contains(&"/static/drivers/{file}"),
            "Missing static asset route"
        );

        // Verify key schemas are present
        let schemas = &spec.components.as_ref().unwrap().schemas;
        assert!(
            schemas.contains_key("GraphqlHttpRequest"),
            "Missing GraphqlHttpRequest schema"
        );
        assert!(
            schemas.contains_key("GraphqlHttpResponse"),
            "Missing GraphqlHttpResponse schema"
        );
        assert!(
            schemas.contains_key("RuesErrorResponse"),
            "Missing RuesErrorResponse schema"
        );
    }

    #[test]
    fn openapi_document_captures_documented_header_and_query_parameters() {
        let spec = serde_json::to_value(axum_app::generated_openapi()).unwrap();

        let ws_doc = spec["paths"]["/on"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            ws_doc.contains("session identifier") || ws_doc.contains("Session"),
            "WebSocket documentation should mention session identifier semantics"
        );

        let graphql_get = operation(&spec, "/graphql", "get");
        let graphql_get_params = parameter_names(graphql_get);
        assert!(
            graphql_get_params.contains("query"),
            "GraphQL GET should document the required query parameter"
        );
        assert!(
            graphql_get_params.contains("operationName"),
            "GraphQL GET should document operationName"
        );
        assert!(
            graphql_get_params.contains("variables"),
            "GraphQL GET should document variables"
        );
        assert!(
            graphql_get_params.contains("extensions"),
            "GraphQL GET should document extensions"
        );

        let subscribe = operation(&spec, "/on/transactions/{topic}", "get");
        let subscribe_params = parameter_names(subscribe);
        assert!(
            subscribe_params.contains("Rusk-Session-Id"),
            "Subscription routes should document the session header"
        );
        assert!(
            subscribe_params.contains("Rusk-Version"),
            "Subscription routes should document Rusk-Version"
        );
        assert!(
            subscribe_params.contains("Rusk-Version-Strict"),
            "Subscription routes should document Rusk-Version-Strict"
        );

        let entity_subscribe =
            operation(&spec, "/on/transactions:{entity}/{topic}", "get");
        let entity_subscribe_params = parameter_names(entity_subscribe);
        assert!(
            entity_subscribe_params.contains("entity"),
            "Entity transaction subscribe should document the entity path parameter"
        );

        let contracts_post =
            operation(&spec, "/on/contracts:{entity}/{topic}", "post");
        let contracts_params = parameter_names(contracts_post);
        assert!(
            contracts_params.contains("Rusk-Feeder"),
            "Contracts POST should document the feeder header"
        );

        let upload_driver =
            operation(&spec, "/on/contract:{entity}/upload_driver", "post");
        let upload_params = parameter_names(upload_driver);
        assert!(
            upload_params.contains("sign"),
            "Driver upload should document the sign header"
        );
    }

    #[test]
    fn openapi_document_captures_status_codes_and_content_types() {
        let spec = serde_json::to_value(axum_app::generated_openapi()).unwrap();

        let network_post = operation(&spec, "/on/network/{topic}", "post");
        let network_statuses = response_statuses(network_post);
        assert!(
            network_statuses.contains("200"),
            "Network POST should document success"
        );
        assert!(
            network_statuses.contains("413"),
            "Network POST should document payload-too-large responses"
        );
        assert!(
            network_statuses.contains("422"),
            "Network POST should document malformed payload/header responses"
        );
        assert!(
            network_statuses.contains("501"),
            "Network POST should document unsupported-topic responses"
        );
        assert!(
            !network_statuses.contains("202"),
            "Network POST should not document Accepted"
        );

        let graphql_get = operation(&spec, "/graphql", "get");
        let graphql_get_statuses = response_statuses(graphql_get);
        assert!(
            graphql_get_statuses.contains("400"),
            "GraphQL GET should document the missing-query error"
        );
        let graphql_get_error_types =
            response_content_types(graphql_get, "400");
        assert!(
            graphql_get_error_types.contains("application/json"),
            "GraphQL GET errors should be documented as JSON"
        );

        let graphql_post = operation(&spec, "/graphql", "post");
        let graphql_post_statuses = response_statuses(graphql_post);
        assert!(
            graphql_post_statuses.contains("413"),
            "GraphQL POST should document payload-too-large responses"
        );
        let graphql_post_error_types =
            response_content_types(graphql_post, "400");
        assert!(
            graphql_post_error_types.contains("application/json"),
            "GraphQL POST errors should be documented as JSON"
        );

        let graphql_legacy = operation(&spec, "/on/graphql/query", "post");
        let graphql_legacy_statuses = response_statuses(graphql_legacy);
        assert!(
            graphql_legacy_statuses.contains("501"),
            "The legacy GraphQL route should document unsupported-handler responses"
        );
        assert!(
            !graphql_legacy_statuses.contains("404"),
            "The legacy GraphQL route should not document GraphQL HTTP not-found responses"
        );
        let graphql_legacy_request_types =
            request_content_types(graphql_legacy);
        assert!(
            graphql_legacy_request_types.contains("text/plain"),
            "The legacy GraphQL route should document raw text requests"
        );
        let graphql_legacy_response_types =
            response_content_types(graphql_legacy, "200");
        assert!(
            graphql_legacy_response_types.contains("application/json"),
            "Legacy GraphQL query responses should be documented as JSON"
        );
        assert!(
            graphql_legacy_response_types.contains("text/plain"),
            "Legacy GraphQL schema responses should be documented as text"
        );

        let contracts_post =
            operation(&spec, "/on/contracts:{entity}/{topic}", "post");
        let contracts_request_types = request_content_types(contracts_post);
        assert!(
            contracts_request_types.contains("application/json"),
            "Contracts POST should document JSON requests"
        );
        assert!(
            contracts_request_types.contains("application/octet-stream"),
            "Contracts POST should document binary requests"
        );

        let upload_driver =
            operation(&spec, "/on/contract:{entity}/upload_driver", "post");
        let upload_statuses = response_statuses(upload_driver);
        assert!(
            upload_statuses.contains("413"),
            "Driver upload should document payload-too-large responses"
        );

        let contract_status =
            operation(&spec, "/on/contract:{entity}/status", "post");
        let contract_statuses = response_statuses(contract_status);
        assert!(
            contract_statuses.contains("404"),
            "Deprecated contract status should document not-found responses"
        );
        assert_eq!(
            contract_status["deprecated"].as_bool(),
            Some(true),
            "Deprecated contract status route should be marked deprecated"
        );

        let transactions_subscribe =
            operation(&spec, "/on/transactions/{topic}", "get");
        let subscribe_statuses = response_statuses(transactions_subscribe);
        assert!(
            !subscribe_statuses.contains("404"),
            "Transaction subscribe should not document not-found responses"
        );

        let transactions_unsubscribe =
            operation(&spec, "/on/transactions/{topic}", "delete");
        let unsubscribe_statuses = response_statuses(transactions_unsubscribe);
        assert!(
            unsubscribe_statuses.contains("404"),
            "Transaction unsubscribe should document missing subscriptions"
        );

        let entity_transactions_unsubscribe =
            operation(&spec, "/on/transactions:{entity}/{topic}", "delete");
        let entity_unsubscribe_statuses =
            response_statuses(entity_transactions_unsubscribe);
        assert!(
            entity_unsubscribe_statuses.contains("404"),
            "Entity transaction unsubscribe should document missing subscriptions"
        );
        assert!(
            operation(&spec, "/on/account:{entity}/{topic}", "post")["deprecated"]
                .as_bool()
                == Some(true),
            "Legacy account route should be marked deprecated"
        );
        assert!(
            operation(&spec, "/on/contract_owner:{entity}/{topic}", "post")
                ["deprecated"]
                .as_bool()
                == Some(true),
            "Legacy contract owner route should be marked deprecated"
        );
        assert!(
            operation(&spec, "/on/contract:{entity}/{topic}", "post")["deprecated"]
                .is_null(),
            "Generic contract route should remain non-deprecated"
        );
    }

    #[test]
    fn openapi_document_uses_split_rues_and_graphql_tags() {
        let spec = serde_json::to_value(axum_app::generated_openapi()).unwrap();

        let tag_names = documented_tag_names(&spec);
        assert!(
            tag_names.contains("RUES"),
            "The RUES WebSocket tag should be documented"
        );
        assert!(
            tag_names.contains("RUES / Dispatch"),
            "The RUES dispatch tag should be documented"
        );
        assert!(
            tag_names.contains("RUES / Events"),
            "The RUES events tag should be documented"
        );
        assert!(
            tag_names.contains("GraphQL"),
            "The GraphQL tag should be documented"
        );

        let graphql_get = operation(&spec, "/graphql", "get");
        assert_eq!(
            operation_tags(graphql_get),
            BTreeSet::from(["GraphQL"]),
            "Canonical GraphQL GET should use the GraphQL tag"
        );

        let graphql_post = operation(&spec, "/graphql", "post");
        assert_eq!(
            operation_tags(graphql_post),
            BTreeSet::from(["GraphQL"]),
            "Canonical GraphQL POST should use the GraphQL tag"
        );

        let graphql_legacy = operation(&spec, "/on/graphql/query", "post");
        assert_eq!(
            operation_tags(graphql_legacy),
            BTreeSet::from(["RUES / Dispatch"]),
            "The `/on` legacy GraphQL route should use the RUES dispatch tag"
        );

        let ws_handshake = operation(&spec, "/on", "get");
        assert_eq!(
            operation_tags(ws_handshake),
            BTreeSet::from(["RUES"]),
            "The RUES WebSocket handshake should use the dedicated RUES tag"
        );

        let transactions_post =
            operation(&spec, "/on/transactions/{topic}", "post");
        assert_eq!(
            operation_tags(transactions_post),
            BTreeSet::from(["RUES / Dispatch"]),
            "Transaction dispatch should use the RUES dispatch tag"
        );

        let transactions_subscribe =
            operation(&spec, "/on/transactions/{topic}", "get");
        assert_eq!(
            operation_tags(transactions_subscribe),
            BTreeSet::from(["RUES / Events"]),
            "Transaction subscribe should use the RUES events tag"
        );

        let transactions_unsubscribe =
            operation(&spec, "/on/transactions/{topic}", "delete");
        assert_eq!(
            operation_tags(transactions_unsubscribe),
            BTreeSet::from(["RUES / Events"]),
            "Transaction unsubscribe should use the RUES events tag"
        );

        let transactions_entity_subscribe =
            operation(&spec, "/on/transactions:{entity}/{topic}", "get");
        assert_eq!(
            operation_tags(transactions_entity_subscribe),
            BTreeSet::from(["RUES / Events"]),
            "Entity transaction subscribe should use the RUES events tag"
        );

        let blocks_entity_subscribe =
            operation(&spec, "/on/blocks:{entity}/{topic}", "get");
        assert_eq!(
            operation_tags(blocks_entity_subscribe),
            BTreeSet::from(["RUES / Events"]),
            "Entity block subscribe should use the RUES events tag"
        );

        let contracts_post =
            operation(&spec, "/on/contracts:{entity}/{topic}", "post");
        assert_eq!(
            operation_tags(contracts_post),
            BTreeSet::from(["RUES / Dispatch"]),
            "Contract calls and queries should use the RUES dispatch tag"
        );

        let contracts_entity_subscribe =
            operation(&spec, "/on/contracts:{entity}/{topic}", "get");
        assert_eq!(
            operation_tags(contracts_entity_subscribe),
            BTreeSet::from(["RUES / Events"]),
            "Contract subscribe should use the RUES events tag"
        );

        let contracts_entity_unsubscribe =
            operation(&spec, "/on/contracts:{entity}/{topic}", "delete");
        assert_eq!(
            operation_tags(contracts_entity_unsubscribe),
            BTreeSet::from(["RUES / Events"]),
            "Contract unsubscribe should use the RUES events tag"
        );
    }
}
