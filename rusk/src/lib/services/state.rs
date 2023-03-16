// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::transaction::TransferPayload;
use crate::{Result, Rusk};

use std::pin::Pin;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::ViewKey;
use futures::{Stream, StreamExt};
use phoenix_core::transaction::StakeData;
use phoenix_core::{Note, Transaction};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::task::LocalPoolHandle;
use tonic::{Request, Response, Status};
use tracing::info;

pub use rusk_schema::state_server::{State, StateServer};
pub use rusk_schema::{
    get_stake_response::Amount, EchoRequest, EchoResponse,
    ExecuteStateTransitionRequest, ExecuteStateTransitionResponse,
    ExecutedTransaction as ExecutedTransactionProto,
    FindExistingNullifiersRequest, FindExistingNullifiersResponse,
    GetAnchorRequest, GetAnchorResponse, GetNotesRequest, GetNotesResponse,
    GetOpeningRequest, GetOpeningResponse, GetProvisionersRequest,
    GetProvisionersResponse, GetStakeRequest, GetStakeResponse,
    GetStateRootRequest, GetStateRootResponse, PersistRequest, PersistResponse,
    PreverifyRequest, PreverifyResponse, Provisioner, RevertRequest,
    RevertResponse, Stake as StakeProto, StateTransitionRequest,
    StateTransitionResponse, Transaction as TransactionProto,
    VerifyStateTransitionRequest, VerifyStateTransitionResponse,
};

#[tonic::async_trait]
impl State for Rusk {
    async fn echo(
        &self,
        request: Request<EchoRequest>,
    ) -> Result<Response<EchoResponse>, Status> {
        info!("Received Echo request");

        let request = request.into_inner();

        Ok(Response::new(EchoResponse {
            message: request.message,
        }))
    }

    async fn preverify(
        &self,
        request: Request<PreverifyRequest>,
    ) -> Result<Response<PreverifyResponse>, Status> {
        info!("Received Preverify request");

        let request = request.into_inner();

        let tx_proto = request.tx.ok_or_else(|| {
            Status::invalid_argument("Transaction is required")
        })?;

        let tx = TransferPayload::from_slice(&tx_proto.payload)
            .map_err(Error::Serialization)?;

        let tx_hash_input_bytes = tx.to_hash_input_bytes();
        let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

        self.pre_verify(&tx)?;

        Ok(Response::new(PreverifyResponse {
            tx_hash: tx_hash.to_bytes().to_vec(),
            fee: Some((tx.fee()).into()),
        }))
    }

    async fn execute_state_transition(
        &self,
        request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        info!("Received ExecuteStateTransition request");

        let request = request.into_inner();

        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;

        let mut discarded_txs = Vec::with_capacity(request.txs.len());

        // Deserialize transactions, collecting failed ones in the
        // `discarded_txs`. This is then appended to with failed transactions.
        let txs = request
            .txs
            .into_iter()
            .filter_map(|tx| match Transaction::from_slice(&tx.payload) {
                Ok(tx) => Some(tx),
                Err(_) => {
                    discarded_txs.push(tx);
                    None
                }
            })
            .collect();

        let (txs, more_discarded_txs, state_root) = self.execute_transactions(
            request.block_height,
            request.block_gas_limit,
            generator,
            txs,
        )?;

        let txs = txs.into_iter().map(|tx| tx.into()).collect();
        let more_discarded_txs: Vec<TransactionProto> =
            more_discarded_txs.into_iter().map(Into::into).collect();

        discarded_txs.extend(more_discarded_txs);

        Ok(Response::new(ExecuteStateTransitionResponse {
            state_root: state_root.to_vec(),
            txs,
            discarded_txs,
        }))
    }

    async fn verify_state_transition(
        &self,
        request: Request<VerifyStateTransitionRequest>,
    ) -> Result<Response<VerifyStateTransitionResponse>, Status> {
        info!("Received VerifyStateTransition request");

        let request = request.into_inner();
        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;

        let txs = deserialize_txs(request.txs)?;

        let (_, state_root) = self.verify_transactions(
            request.block_height,
            request.block_gas_limit,
            generator,
            txs,
        )?;

        Ok(Response::new(VerifyStateTransitionResponse {
            state_root: state_root.to_vec(),
        }))
    }

    async fn accept(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        info!("Received Accept request");

        let request = request.into_inner();
        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;

        let txs = deserialize_txs(request.txs)?;

        let (txs, state_root) = self.accept_transactions(
            request.block_height,
            request.block_gas_limit,
            generator,
            txs,
        )?;

        let txs = txs.into_iter().map(Into::into).collect();

        Ok(Response::new(StateTransitionResponse {
            txs,
            state_root: state_root.to_vec(),
        }))
    }

    async fn finalize(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        info!("Received Finalize request");

        let request = request.into_inner();
        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;

        let txs = deserialize_txs(request.txs)?;

        let (txs, state_root) = self.finalize_transactions(
            request.block_height,
            request.block_gas_limit,
            generator,
            txs,
        )?;

        let txs = txs.into_iter().map(Into::into).collect();

        Ok(Response::new(StateTransitionResponse {
            txs,
            state_root: state_root.to_vec(),
        }))
    }

    async fn revert(
        &self,
        _request: Request<RevertRequest>,
    ) -> Result<Response<RevertResponse>, Status> {
        info!("Received Revert request");

        let commit_id = self.revert()?;
        let state_root = commit_id.to_vec();

        Ok(Response::new(RevertResponse { state_root }))
    }

    async fn persist(
        &self,
        request: Request<PersistRequest>,
    ) -> Result<Response<PersistResponse>, Status> {
        info!("Received Persist request");

        let request = request.into_inner();

        let state_root = self.state_root();

        if request.state_root != state_root {
            return Err(Status::invalid_argument(format!(
                "state root mismatch. Expected {}, Got {}",
                hex::encode(state_root),
                hex::encode(request.state_root)
            )));
        }

        self.persist_state()?;

        Ok(Response::new(PersistResponse {}))
    }

    async fn get_provisioners(
        &self,
        _request: Request<GetProvisionersRequest>,
    ) -> Result<Response<GetProvisionersResponse>, Status> {
        info!("Received GetProvisioners request");

        let provisioners = self
            .provisioners()?
            .into_iter()
            .filter_map(|(key, stake)| {
                stake.amount.map(|(value, eligibility)| {
                    let raw_public_key_bls = key.to_raw_bytes().to_vec();
                    let public_key_bls = key.to_bytes().to_vec();

                    let stake = StakeProto {
                        value,
                        eligibility,
                        reward: stake.reward,
                        counter: stake.counter,
                    };

                    Provisioner {
                        raw_public_key_bls,
                        public_key_bls,
                        stakes: vec![stake],
                    }
                })
            })
            .collect();

        Ok(Response::new(GetProvisionersResponse { provisioners }))
    }

    async fn get_state_root(
        &self,
        _request: Request<GetStateRootRequest>,
    ) -> Result<Response<GetStateRootResponse>, Status> {
        info!("Received GetEphemeralStateRoot request");

        let state_root = self.state_root();
        Ok(Response::new(GetStateRootResponse {
            state_root: state_root.to_vec(),
        }))
    }

    type GetNotesStream =
        Pin<Box<dyn Stream<Item = Result<GetNotesResponse, Status>> + Send>>;

    async fn get_notes(
        &self,
        request: Request<GetNotesRequest>,
    ) -> Result<Response<Self::GetNotesStream>, Status> {
        info!("Received GetNotes request");

        let request = request.into_inner();

        let vk = match request.vk.is_empty() {
            false => {
                let vk = ViewKey::from_slice(&request.vk)
                    .map_err(Error::Serialization)?;
                Some(vk)
            }
            true => None,
        };

        let (sender, receiver) = mpsc::channel(self.stream_buffer_size);

        // Clone rusk and move it to the thread
        let rusk = self.clone();

        // Spawn a task that's responsible for iterating through the leaves of
        // the transfer contract tree and sending them through the sender
        spawn(async move {
            let local_pool = LocalPoolHandle::new(1);
            local_pool
                .spawn_pinned(move || async move {
                    const BLOCKS_TO_SEARCH: u64 = 16;

                    let mut start_height = request.height;

                    loop {
                        let end_height = start_height + BLOCKS_TO_SEARCH;

                        let leaves = rusk
                            .leaves_in_range(start_height..end_height)
                            .expect("failed to iterate through leaves");

                        if leaves.is_empty() {
                            break;
                        }

                        for leaf in leaves {
                            if let Some(vk) = vk {
                                if !vk.owns(&leaf.note) {
                                    continue;
                                }
                            }

                            if sender.send(leaf).await.is_err() {
                                break;
                            }
                        }

                        start_height += BLOCKS_TO_SEARCH;
                    }
                })
                .await
        });

        // Make a stream from the receiver and map the elements to be the
        // expected output
        let stream = ReceiverStream::new(receiver).map(|leaf| {
            Ok(GetNotesResponse {
                note: leaf.note.to_bytes().to_vec(),
                height: leaf.block_height,
            })
        });

        Ok(Response::new(Box::pin(stream) as Self::GetNotesStream))
    }

    async fn get_anchor(
        &self,
        _request: Request<GetAnchorRequest>,
    ) -> Result<Response<GetAnchorResponse>, Status> {
        info!("Received GetAnchor request");

        let anchor = self.tree_root()?;
        Ok(Response::new(GetAnchorResponse {
            anchor: anchor.to_bytes().to_vec(),
        }))
    }

    async fn get_opening(
        &self,
        request: Request<GetOpeningRequest>,
    ) -> Result<Response<GetOpeningResponse>, Status> {
        info!("Received GetOpening request");

        let note = Note::from_slice(&request.get_ref().note)
            .map_err(Error::Serialization)?;

        let branch = self
            .tree_opening(*note.pos())?
            .ok_or(Status::invalid_argument("No such opening"))?;

        Ok(Response::new(GetOpeningResponse {
            branch: branch.to_bytes().to_vec(),
        }))
    }

    async fn get_stake(
        &self,
        request: Request<GetStakeRequest>,
    ) -> Result<Response<GetStakeResponse>, Status> {
        info!("Received GetStake request");

        const ERR: Error = Error::Serialization(dusk_bytes::Error::InvalidData);

        let mut bytes = [0u8; PublicKey::SIZE];

        let pk = request.get_ref().pk.as_slice();

        if pk.len() < PublicKey::SIZE {
            return Err(ERR.into());
        }

        (bytes[..PublicKey::SIZE]).copy_from_slice(&pk[..PublicKey::SIZE]);

        let pk = PublicKey::from_bytes(&bytes).map_err(|_| ERR)?;

        let stake = self.stake(pk)?.unwrap_or(StakeData {
            amount: None,
            reward: 0,
            counter: 0,
        });

        let amount = stake
            .amount
            .map(|(value, eligibility)| Amount { value, eligibility });

        Ok(Response::new(GetStakeResponse {
            amount,
            reward: stake.reward,
            counter: stake.counter,
        }))
    }

    async fn find_existing_nullifiers(
        &self,
        request: Request<FindExistingNullifiersRequest>,
    ) -> Result<Response<FindExistingNullifiersResponse>, Status> {
        info!("Received FindExistingNullifiers request");

        let nullifiers = &request.get_ref().nullifiers;

        let nullifiers = nullifiers
            .iter()
            .map(|n| BlsScalar::from_slice(n).map_err(Error::Serialization))
            .collect::<Result<Vec<_>, _>>()?;

        let nullifiers = self.existing_nullifiers(&nullifiers)?;

        let nullifiers =
            nullifiers.iter().map(|n| n.to_bytes().to_vec()).collect();

        Ok(Response::new(FindExistingNullifiersResponse { nullifiers }))
    }
}

fn deserialize_txs(
    txs: Vec<TransactionProto>,
) -> Result<Vec<Transaction>, Status> {
    txs.into_iter()
        .map(|tx| {
            Transaction::from_slice(&tx.payload).map_err(|_| {
                Status::invalid_argument("Transaction deserialization failed")
            })
        })
        .collect()
}
