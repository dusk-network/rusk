use std::path::{Path, PathBuf};

use phoenix::{rpc, DbRoot, Transaction};
use tracing::trace;

pub struct RuskServer {
    db_path: PathBuf,
}

impl RuskServer {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            db_path: path.as_ref().to_path_buf(),
        }
    }
}

#[tonic::async_trait]
impl rpc::rusk_server::Rusk for RuskServer {
    async fn echo(
        &self,
        _request: tonic::Request<rpc::EchoRequest>,
    ) -> Result<tonic::Response<rpc::EchoResponse>, tonic::Status> {
        trace!("Incoming echo request");
        Ok(tonic::Response::new(rpc::EchoResponse::default()))
    }

    async fn validate_state_transition(
        &self,
        request: tonic::Request<rpc::ValidateStateTransitionRequest>,
    ) -> Result<tonic::Response<rpc::ValidateStateTransitionResponse>, tonic::Status> {
        let mut txs = vec![];
        for t in request.into_inner().txs {
            txs.push(
                Transaction::try_from_rpc_transaction(&self.db_path, t)
                    .map_err(|e| tonic::Status::invalid_argument(e.to_string()))?,
            );
        }

        let state = DbRoot::new(&self.db_path)
            .and_then(|root| root.restore())
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        state
            .validate_bulk_transaction(txs.as_slice())
            .map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;

        Ok(tonic::Response::new(rpc::ValidateStateTransitionResponse {
            success: true,
        }))
    }
}
