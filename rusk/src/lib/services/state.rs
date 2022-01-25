// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::Rusk;

use canonical::{Canon, Sink};
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::{PublicKey, ViewKey};
use dusk_wallet_core::Transaction;
use phoenix_core::Note;
use tonic::{Request, Response, Status};
use tracing::info;

use rusk_vm::GasMeter;

pub use super::rusk_proto::state_server::{State, StateServer};
pub use super::rusk_proto::{
    EchoRequest, EchoResponse, ExecuteStateTransitionRequest,
    ExecuteStateTransitionResponse, GetAnchorRequest, GetAnchorResponse,
    GetNotesOwnedByRequest, GetNotesOwnedByResponse, GetOpeningRequest,
    GetOpeningResponse, GetProvisionersRequest, GetProvisionersResponse,
    GetStakeRequest, GetStakeResponse, GetStateRootRequest,
    GetStateRootResponse, Transaction as TransactionProto,
    VerifyStateTransitionRequest, VerifyStateTransitionResponse,
};

#[tonic::async_trait]
impl State for Rusk {
    async fn echo(
        &self,
        _request: Request<EchoRequest>,
    ) -> Result<Response<EchoResponse>, Status> {
        info!("Received Echo request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn execute_state_transition(
        &self,
        request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        info!("Received ExecuteStateTransition request");

        let mut state = self.state()?;
        let network = state.inner_mut();
        let mut block_gas_meter =
            GasMeter::with_limit(request.get_ref().block_gas_limit);

        let txs: Vec<_> = request
            .get_ref()
            .txs
            .iter()
            .map(|tx| Transaction::from_slice(&tx.payload))
            .filter_map(|tx| tx.ok())
            .map(|tx| {
                let block_height = request.get_ref().block_height;
                let mut gas_meter = GasMeter::with_limit(tx.fee().gas_limit);

                let _ = network.transact::<_, ()>(
                    rusk_abi::transfer_contract(),
                    block_height,
                    tx.clone(),
                    &mut gas_meter,
                );
                (tx, gas_meter)
            })
            .take_while(|(_, gas_meter)| {
                block_gas_meter.charge(gas_meter.spent()).is_ok()
            })
            .map(|(tx, _)| {
                let payload = tx.to_bytes();

                TransactionProto {
                    version: 1,
                    r#type: 1,
                    payload,
                }
            })
            .collect();

        let success = true;
        let state_root = network.root().to_vec();

        Ok(Response::new(ExecuteStateTransitionResponse {
            success,
            txs,
            state_root,
        }))
    }

    async fn verify_state_transition(
        &self,
        request: Request<VerifyStateTransitionRequest>,
    ) -> Result<Response<VerifyStateTransitionResponse>, Status> {
        info!("Received VerifyStateTransition request");

        let mut state = self.state()?;
        let network = state.inner_mut();
        let mut block_gas_meter =
            GasMeter::with_limit(request.get_ref().block_gas_limit);

        let success = request
            .get_ref()
            .txs
            .iter()
            .map(|tx| Transaction::from_slice(&tx.payload))
            .all(|tx| match tx {
                Ok(tx) => {
                    let block_height = request.get_ref().block_height;
                    let mut gas_meter =
                        GasMeter::with_limit(tx.fee().gas_limit);

                    network
                        .transact::<_, ()>(
                            rusk_abi::transfer_contract(),
                            block_height,
                            tx,
                            &mut gas_meter,
                        )
                        .is_ok()
                        && block_gas_meter.charge(gas_meter.spent()).is_ok()
                }
                Err(_) => false,
            });

        Ok(Response::new(VerifyStateTransitionResponse { success }))
    }

    async fn accept(
        &self,
        request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        info!("Received Accept request");

        let response = self.execute_state_transition(request).await;

        self.persist()?;

        response
    }

    async fn finalize(
        &self,
        request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        let response = self.execute_state_transition(request).await;

        self.state()?.commit();
        self.persist()?;

        response
    }

    async fn get_provisioners(
        &self,
        _request: Request<GetProvisionersRequest>,
    ) -> Result<Response<GetProvisionersResponse>, Status> {
        info!("Received GetProvisioners request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn get_state_root(
        &self,
        _request: Request<GetStateRootRequest>,
    ) -> Result<Response<GetStateRootResponse>, Status> {
        info!("Received GetEphemeralStateRoot request");

        let state_root = self.state()?.root().to_vec();
        Ok(Response::new(GetStateRootResponse { state_root }))
    }

    async fn get_notes_owned_by(
        &self,
        request: Request<GetNotesOwnedByRequest>,
    ) -> Result<Response<GetNotesOwnedByResponse>, Status> {
        info!("Received GetNotesOwnedBy request");

        let vk = ViewKey::from_slice(&request.get_ref().vk)
            .map_err(Error::Serialization)?;

        let notes = self
            .state()?
            .fetch_notes(request.get_ref().height, &vk)?
            .iter()
            .map(|n| n.to_bytes().to_vec())
            .collect();
        Ok(Response::new(GetNotesOwnedByResponse { notes }))
    }

    async fn get_anchor(
        &self,
        _request: Request<GetAnchorRequest>,
    ) -> Result<Response<GetAnchorResponse>, Status> {
        info!("Received GetAnchor request");

        let anchor = self.state()?.fetch_anchor()?.to_bytes().to_vec();
        Ok(Response::new(GetAnchorResponse { anchor }))
    }

    async fn get_opening(
        &self,
        request: Request<GetOpeningRequest>,
    ) -> Result<Response<GetOpeningResponse>, Status> {
        info!("Received GetOpening request");

        let note = Note::from_slice(&request.get_ref().note)
            .map_err(Error::Serialization)?;

        let branch = self.state()?.fetch_opening(&note)?;

        const PAGE_SIZE: usize = 1024 * 64;
        let mut bytes = [0u8; PAGE_SIZE];
        let mut sink = Sink::new(&mut bytes[..]);
        branch.encode(&mut sink);
        let len = branch.encoded_len();
        let branch = (&bytes[..len]).to_vec();

        Ok(Response::new(GetOpeningResponse { branch }))
    }

    async fn get_stake(
        &self,
        request: Request<GetStakeRequest>,
    ) -> Result<Response<GetStakeResponse>, Status> {
        info!("Received GetStake request");

        let pk = PublicKey::from_slice(&request.get_ref().pk)
            .map_err(Error::Serialization)?;

        let stake = self.state()?.fetch_stake(&pk)?;

        let (stake, expiration) = (stake.value(), stake.expiration());
        Ok(Response::new(GetStakeResponse { stake, expiration }))
    }
}
