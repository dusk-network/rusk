// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::{Result, Rusk, RuskState};

use canonical::{Canon, Sink};
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::{PublicKey, ViewKey};
use dusk_wallet_core::Transaction;
use phoenix_core::Note;
use tonic::{Request, Response, Status};
use tracing::info;

use rusk_vm::{GasMeter, NetworkState};

pub use super::rusk_proto::state_server::{State, StateServer};
pub use super::rusk_proto::{
    EchoRequest, EchoResponse, ExecuteStateTransitionRequest,
    ExecuteStateTransitionResponse, GetAnchorRequest, GetAnchorResponse,
    GetNotesOwnedByRequest, GetNotesOwnedByResponse, GetOpeningRequest,
    GetOpeningResponse, GetProvisionersRequest, GetProvisionersResponse,
    GetStakeRequest, GetStakeResponse, GetStateRootRequest,
    GetStateRootResponse, StateTransitionRequest, StateTransitionResponse,
    Transaction as TransactionProto, VerifyStateTransitionRequest,
    VerifyStateTransitionResponse,
};

const TX_VERSION: u32 = 1;
const TX_TYPE_COINBASE: u32 = 0;
const TX_TYPE_TRANSFER: u32 = 1;

/// Partition transactions into transfer and coinbase transactions, in this
/// order.
fn extract_coinbase(
    tx: Vec<TransactionProto>,
) -> Result<(Vec<TransactionProto>, (Note, Note)), Status> {
    let (transfer_txs, coinbase_txs): (Vec<_>, Vec<_>) =
        tx.into_iter().partition(|tx| tx.r#type == TX_TYPE_TRANSFER);

    // There must always be two Coinbase transactions
    let coinbases = coinbase_txs.len();
    if coinbases == 2 {
        return Err(Status::invalid_argument(format!(
            "Expected 2 coinbase transactions, got {}",
            coinbases
        )));
    }

    let dusk_note = Note::from_slice(&coinbase_txs[0].payload)
        .map_err(Error::Serialization)?;
    let generator_note = Note::from_slice(&coinbase_txs[1].payload)
        .map_err(Error::Serialization)?;

    Ok((transfer_txs, (dusk_note, generator_note)))
}

impl Rusk {
    fn accept_transactions(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<(Response<StateTransitionResponse>, RuskState), Status> {
        let request = request.into_inner();

        let mut state = self.state()?;
        let network = state.inner_mut();
        let mut block_gas_meter = GasMeter::with_limit(request.block_gas_limit);

        let (transfer_txs, coinbase) = extract_coinbase(request.txs)?;

        let txs = self.execute_transactions(
            network,
            &mut block_gas_meter,
            request.block_height,
            &transfer_txs,
        );

        state.push_coinbase(
            request.block_height,
            block_gas_meter.spent(),
            coinbase,
        )?;
        let state_root = state.root().to_vec();

        Ok((
            Response::new(StateTransitionResponse { txs, state_root }),
            state,
        ))
    }

    fn execute_transactions(
        &self,
        network: &mut NetworkState,
        block_gas_meter: &mut GasMeter,
        block_height: u64,
        txs: &[TransactionProto],
    ) -> Vec<TransactionProto> {
        txs.iter()
            .map(|tx| Transaction::from_slice(&tx.payload))
            .filter_map(|tx| tx.ok())
            .map(|tx| {
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
                    version: TX_VERSION,
                    r#type: TX_TYPE_TRANSFER,
                    payload,
                }
            })
            .collect()
    }
}

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

        let request = request.into_inner();

        let mut state = self.state()?;
        let network = state.inner_mut();
        let mut block_gas_meter = GasMeter::with_limit(request.block_gas_limit);

        let mut txs = self.execute_transactions(
            network,
            &mut block_gas_meter,
            request.block_height,
            &request.txs,
        );

        let (dusk_note, generator_note) = state.mint(
            request.block_height,
            block_gas_meter.spent(),
            self.generator.as_ref(),
        )?;

        for note in [dusk_note, generator_note] {
            txs.push(TransactionProto {
                version: TX_VERSION,
                r#type: TX_TYPE_COINBASE,
                payload: note.to_bytes().to_vec(),
            })
        }

        let success = true;
        let state_root = state.root().to_vec();

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

        let request = request.into_inner();

        let mut state = self.state()?;
        let network = state.inner_mut();
        let mut block_gas_meter = GasMeter::with_limit(request.block_gas_limit);

        let (transfer_txs, coinbase) = extract_coinbase(request.txs)?;

        let success = transfer_txs
            .iter()
            .map(|tx| Transaction::from_slice(&tx.payload))
            .all(|tx| match tx {
                Ok(tx) => {
                    let block_height = request.block_height;
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

        if !success {
            return Ok(Response::new(VerifyStateTransitionResponse {
                success: false,
            }));
        }

        state.push_coinbase(
            request.block_height,
            block_gas_meter.spent(),
            coinbase,
        )?;

        Ok(Response::new(VerifyStateTransitionResponse { success }))
    }

    async fn accept(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        info!("Received Accept request");

        let (response, mut state) = self.accept_transactions(request)?;

        self.persist(&mut state)?;

        Ok(response)
    }

    async fn finalize(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        info!("Received Finalize request");

        let (response, mut state) = self.accept_transactions(request)?;

        state.commit();
        self.persist(&mut state)?;

        Ok(response)
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
