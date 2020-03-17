use std::path::{Path, PathBuf};

use phoenix::{db, rpc, Transaction};
use tracing::trace;

pub struct Rusk {
    db_path: PathBuf,
}

impl Rusk {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            db_path: path.as_ref().to_path_buf(),
        }
    }
}

#[tonic::async_trait]
impl rpc::rusk_server::Rusk for Rusk {
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
    ) -> Result<
        tonic::Response<rpc::ValidateStateTransitionResponse>,
        tonic::Status,
    > {
        let mut txs = vec![];
        for t in request.into_inner().txs {
            txs.push(
                Transaction::try_from_rpc_transaction(&self.db_path, t)
                    .map_err(|e| {
                        tonic::Status::invalid_argument(e.to_string())
                    })?,
            );
        }

        db::store_bulk_transactions(&self.db_path, &txs)
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        println!("it happened");

        Ok(tonic::Response::new(rpc::ValidateStateTransitionResponse {
            success: true,
        }))
    }
}
