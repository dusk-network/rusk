use async_graphql::ServerError;
use async_graphql::Value;
use hyper::StatusCode;
use thiserror::Error;
use tracing::{debug, error};

pub type ChainResult<T> = std::result::Result<T, ChainError>;

/// Chain server error types
#[derive(Debug, Error)]
pub enum ChainError {
    #[error("{message}")]
    Custom { message: String },
    #[error("{0:?}")]
    GraphQL(Vec<ServerError>, StatusCode),
    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid Data")]
    Bytes,
    #[error("Invalid Transaction: {0}")]
    TxAcceptanceError(#[from] node::mempool::TxAcceptanceError),
    #[error("{0}")]
    Request(#[from] reqwest::Error),
}

impl ChainError {
    pub fn status(&self) -> StatusCode {
        match self {
            ChainError::GraphQL(_, status) => *status,
            ChainError::TxAcceptanceError(_) => StatusCode::BAD_REQUEST,
            ChainError::Json(_)
            | ChainError::Bytes
            | ChainError::Request(_)
            | ChainError::Custom { .. } => {
                error!("Internal Server Error: {}", self);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl From<Vec<ServerError>> for ChainError {
    fn from(errors: Vec<ServerError>) -> Self {
        // Vec<ServerError> contains our raised FieldErrors in an error case.
        // Our Errors must have a status code, so we can check it.
        // Any other case will be a 500 error.
        match errors.len() {
            0 => {
                error!("Empty error list in GraphQL response");
                ChainError::Custom {
                    message: "Empty error list in GraphQL response".to_string(),
                }
            }
            _ => {
                debug!("Found {} GraphQL errors", errors.len());

                let error = errors[0].clone();
                let status = if let Some(extensions) = error.extensions {
                    extensions.get("status").map(|s| match s {
                        Value::Number(n) => n.as_u64().unwrap_or(500) as u16,
                        _ => 500,
                    })
                } else {
                    None
                }
                .unwrap_or(500);

                ChainError::GraphQL(
                    errors,
                    StatusCode::from_u16(status).unwrap_or_else(|_| {
                        error!("Invalid status code: {}", status);
                        StatusCode::INTERNAL_SERVER_ERROR
                    }),
                )
            }
        }
    }
}
