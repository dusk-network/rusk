// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::services::prover::RuskProver;
use crate::transaction::{SpentTransaction, TransferPayload};
use crate::{Result, Rusk, RuskState};

use std::collections::BTreeSet;
use std::pin::Pin;

use canonical::{Canon, Sink};
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::ViewKey;
use futures::{Stream, StreamExt};
use phoenix_core::Note;
use rusk_vm::GasMeter;
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
    GetAnchorRequest, GetAnchorResponse, GetNotesOwnedByRequest,
    GetNotesOwnedByResponse, GetNotesRequest, GetNotesResponse,
    GetOpeningRequest, GetOpeningResponse, GetProvisionersRequest,
    GetProvisionersResponse, GetStakeRequest, GetStakeResponse,
    GetStateRootRequest, GetStateRootResponse, PersistRequest, PersistResponse,
    PreverifyRequest, PreverifyResponse, Provisioner, RevertRequest,
    RevertResponse, Stake as StakeProto, StateTransitionRequest,
    StateTransitionResponse, Transaction as TransactionProto,
    VerifyStateTransitionRequest, VerifyStateTransitionResponse,
};
use uuid::Uuid;

impl Rusk {
    fn verify(&self, tx: &TransferPayload, uuid: Uuid) -> Result<(), Status> {
        if self.state(uuid)?.any_nullifier_exists(tx.inputs())? {
            return Err(Status::failed_precondition(
                "Nullifier(s) already exists in the state",
            ));
        }

        if !RuskProver::preverify(tx)? {
            return Err(Status::failed_precondition(
                "Proof verification failed",
            ));
        }

        Ok(())
    }

    fn accept_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        transfer_txs: Vec<TransactionProto>,
        generator: PublicKey,
        uuid: Uuid,
    ) -> Result<(Response<StateTransitionResponse>, RuskState), Status> {
        let mut state = self.state(uuid)?;
        let mut block_gas_left = block_gas_limit;

        let mut txs = Vec::with_capacity(transfer_txs.len());
        let mut dusk_spent = 0;

        let mut nullifiers = BTreeSet::new();

        for tx in transfer_txs {
            let tx = TransferPayload::from_slice(&tx.payload)
                .map_err(Error::Serialization)?;

            for input in tx.inputs() {
                if !nullifiers.insert(*input) {
                    return Err(Status::invalid_argument(format!(
                        "Repeated nullifier: {:x?}",
                        input
                    )));
                }
            }

            let gas_limit = tx.fee().gas_limit;

            let mut gas_meter = GasMeter::with_limit(gas_limit);

            let result =
                state.execute::<()>(block_height, tx.clone(), &mut gas_meter);

            dusk_spent += gas_meter.spent() * tx.fee().gas_price;

            block_gas_left = block_gas_left
                .checked_sub(gas_meter.spent())
                .ok_or_else(|| Status::invalid_argument("Out of gas"))?;

            let spent_tx = SpentTransaction(tx, gas_meter, result.err());
            txs.push(spent_tx.into());
        }

        state.push_coinbase(block_height, dusk_spent, &generator)?;
        let state_root = state.root().to_vec();

        Ok((
            Response::new(StateTransitionResponse { txs, state_root }),
            state,
        ))
    }
}

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
        let uuid = Uuid::new_v4();
        info!("Received Preverify request {}", uuid);

        let request = request.into_inner();

        let tx_proto = request.tx.ok_or_else(|| {
            Status::invalid_argument("Transaction is required")
        })?;

        let tx = TransferPayload::from_slice(&tx_proto.payload)
            .map_err(Error::Serialization)?;

        let tx_hash = tx.hash();

        self.verify(&tx, uuid)?;
        info!("Finishing Preverify request {}", uuid);

        Ok(Response::new(PreverifyResponse {
            tx_hash: tx_hash.to_bytes().to_vec(),
            fee: Some((tx.fee()).into()),
        }))
    }

    async fn execute_state_transition(
        &self,
        request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received ExecuteStateTransition request {}", uuid);

        let mut state = self.state(uuid)?;

        let request = request.into_inner();

        let mut block_gas_left = request.block_gas_limit;

        let mut txs = Vec::with_capacity(request.txs.len());
        let mut discarded_txs = Vec::with_capacity(request.txs.len());

        let mut dusk_spent = 0;

        let mut nullifiers = BTreeSet::new();

        // Here we discard transactions that:
        // - Fail parsing
        // - Use nullifiers that are already use by previous TXs
        // - Spend more gas than the running `block_gas_left`
        'tx_loop: for req_tx in request.txs {
            if let Ok(tx) = TransferPayload::from_slice(&req_tx.payload) {
                for input in tx.inputs() {
                    if !nullifiers.insert(*input) {
                        discarded_txs.push(req_tx);
                        continue 'tx_loop;
                    }
                }

                let mut forked_state = state.clone();
                let mut gas_meter = GasMeter::with_limit(tx.fee().gas_limit);

                // We do not care if the transaction fails or succeeds here
                let result = forked_state.execute::<()>(
                    request.block_height,
                    tx.clone(),
                    &mut gas_meter,
                );

                let gas_spent = gas_meter.spent();

                // If the transaction executes with more gas than is left in the
                // block reject it
                if gas_spent > block_gas_left {
                    continue;
                }

                block_gas_left -= gas_spent;
                dusk_spent += gas_spent * tx.fee().gas_price;

                state = forked_state;
                let spent_tx = SpentTransaction(tx, gas_meter, result.err());
                txs.push(spent_tx.into());

                // No need to keep executing if there is no gas left in the
                // block
                if block_gas_left == 0 {
                    break;
                }
            }
        }

        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;

        state.push_coinbase(request.block_height, dusk_spent, &generator)?;

        // Compute the new state root resulting from the state changes
        let state_root = state.root().to_vec();

        info!("Finishing ExecuteStateTransition request {}", uuid);
        Ok(Response::new(ExecuteStateTransitionResponse {
            state_root,
            txs,
            discarded_txs,
        }))
    }

    async fn verify_state_transition(
        &self,
        request: Request<VerifyStateTransitionRequest>,
    ) -> Result<Response<VerifyStateTransitionResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received VerifyStateTransition request {}", uuid);

        let request = request.into_inner();
        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;
        let (response, _) = self.accept_transactions(
            request.block_height,
            request.block_gas_limit,
            request.txs,
            generator,
            uuid,
        )?;

        let state_root = response.get_ref().state_root.to_vec();
        info!("Finishing VerifyStateTransition request {}", uuid);

        Ok(Response::new(VerifyStateTransitionResponse { state_root }))
    }

    async fn accept(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received Accept request {}", uuid);

        let request = request.into_inner();
        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;
        let (response, mut state) = self.accept_transactions(
            request.block_height,
            request.block_gas_limit,
            request.txs,
            generator,
            uuid,
        )?;

        state.accept();

        info!("Finishing Accept request {}", uuid);

        Ok(response)
    }

    async fn finalize(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received Finalize request {}", uuid);

        let request = request.into_inner();
        let generator = PublicKey::from_slice(&request.generator)
            .map_err(Error::Serialization)?;
        let (response, mut state) = self.accept_transactions(
            request.block_height,
            request.block_gas_limit,
            request.txs,
            generator,
            uuid,
        )?;

        state.finalize();
        info!("Finishing Finalize request {}", uuid);
        Ok(response)
    }

    async fn revert(
        &self,
        _request: Request<RevertRequest>,
    ) -> Result<Response<RevertResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received Revert request {}", uuid);

        let mut state = self.state(uuid)?;
        state.revert();

        let state_root = state.root().to_vec();
        info!("Finishing Revert request {}", uuid);
        Ok(Response::new(RevertResponse { state_root }))
    }

    async fn persist(
        &self,
        request: Request<PersistRequest>,
    ) -> Result<Response<PersistResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received Persist request, {}", uuid);

        let request = request.into_inner();

        let mut state = self.state(uuid)?;
        let state_root = state.root();

        if request.state_root != state_root {
            return Err(Status::invalid_argument(format!(
                "state root mismatch. Expected {}, Got {}",
                hex::encode(state_root),
                hex::encode(request.state_root)
            )));
        }

        self.persist(&mut state)?;
        info!("Finishing Persist request {}", uuid);
        Ok(Response::new(PersistResponse {}))
    }

    async fn get_provisioners(
        &self,
        _request: Request<GetProvisionersRequest>,
    ) -> Result<Response<GetProvisionersResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received GetProvisioners request {}", uuid);

        let state = self.state(uuid)?;
        let provisioners = state
            .get_provisioners()?
            .into_iter()
            .filter_map(|(key, stake)| {
                stake.amount().copied().map(|(value, eligibility)| {
                    let raw_public_key_bls = key.to_raw_bytes().to_vec();
                    let public_key_bls = key.to_bytes().to_vec();

                    let stake = StakeProto {
                        value,
                        eligibility,
                        reward: stake.reward(),
                        counter: stake.counter(),
                    };

                    Provisioner {
                        raw_public_key_bls,
                        public_key_bls,
                        stakes: vec![stake],
                    }
                })
            })
            .collect();
        info!("Finishing GetProvisioners request {}", uuid);
        Ok(Response::new(GetProvisionersResponse { provisioners }))
    }

    async fn get_state_root(
        &self,
        _request: Request<GetStateRootRequest>,
    ) -> Result<Response<GetStateRootResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received GetEphemeralStateRoot request {}", uuid);

        let state_root = self.state(uuid)?.root().to_vec();
        info!("Finishing GetEphemeralStateRoot request {}", uuid);
        Ok(Response::new(GetStateRootResponse { state_root }))
    }

    type GetNotesStream =
        Pin<Box<dyn Stream<Item = Result<GetNotesResponse, Status>> + Send>>;

    async fn get_notes(
        &self,
        request: Request<GetNotesRequest>,
    ) -> Result<Response<Self::GetNotesStream>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received GetNotes request {}", uuid);

        let request = request.into_inner();

        let vk = match request.vk.is_empty() {
            false => {
                let vk = ViewKey::from_slice(&request.vk)
                    .map_err(Error::Serialization)?;
                Some(vk)
            }
            true => None,
        };

        let state = self.state(uuid)?;
        let transfer = state.transfer_contract().map_err(Error::from)?;

        let (sender, receiver) = mpsc::channel(self.stream_buffer_size);

        // Spawn a task that's responsible for iterating through the leaves of
        // the transfer contract tree and sending them through the sender
        spawn(async move {
            let local_pool = LocalPoolHandle::new(1);
            local_pool
                .spawn_pinned(move || async move {
                    let mut leaves_iter = transfer
                        .leaves_from_height(request.height)
                        .expect("Failed iterating through leaves")
                        .map(|item| item.map(|leaf| *leaf));

                    for item in leaves_iter.by_ref() {
                        // Filter out the notes that are not owned by the given
                        // view key(if it was given)
                        if let Some(vk) = vk {
                            if let Ok(leaf) = item.as_ref() {
                                if !vk.owns(&leaf.note) {
                                    continue;
                                }
                            }
                        }

                        if sender.send(item).await.is_err() {
                            break;
                        }
                    }
                    info!("Finishing GetNotes stream request {}", uuid);
                })
                .await
        });

        // Make a stream from the receiver and map the elements to be the
        // expected output
        let stream = ReceiverStream::new(receiver).map(|item| {
            item.map(|leaf| GetNotesResponse {
                note: leaf.note.to_bytes().to_vec(),
                height: leaf.block_height.into(),
            })
            .map_err(|_| {
                Status::internal("Failed iterating through the poseidon tree")
            })
        });
        info!("Finishing GetNotes init request {}", uuid);
        Ok(Response::new(Box::pin(stream) as Self::GetNotesStream))
    }

    #[allow(deprecated)]
    async fn get_notes_owned_by(
        &self,
        request: Request<GetNotesOwnedByRequest>,
    ) -> Result<Response<GetNotesOwnedByResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received GetNotesOwnedBy request {}", uuid);

        let vk = ViewKey::from_slice(&request.get_ref().vk)
            .map_err(Error::Serialization)?;
        let block_height = request.get_ref().height;

        let state = self.state(uuid)?;

        let (notes, height) = state.fetch_notes(block_height, &vk)?;
        let notes = notes.iter().map(|note| note.to_bytes().to_vec()).collect();

        info!("Finishing GetNotesOwnedBy request {}", uuid);
        Ok(Response::new(GetNotesOwnedByResponse { notes, height }))
    }

    async fn get_anchor(
        &self,
        _request: Request<GetAnchorRequest>,
    ) -> Result<Response<GetAnchorResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received GetAnchor request {}", uuid);

        let anchor = self.state(uuid)?.fetch_anchor()?.to_bytes().to_vec();
        info!("Finishing GetAnchor request {}", uuid);
        Ok(Response::new(GetAnchorResponse { anchor }))
    }

    async fn get_opening(
        &self,
        request: Request<GetOpeningRequest>,
    ) -> Result<Response<GetOpeningResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received GetOpening request {}", uuid);

        let note = Note::from_slice(&request.get_ref().note)
            .map_err(Error::Serialization)?;

        let branch = self.state(uuid)?.fetch_opening(&note)?;

        const PAGE_SIZE: usize = 1024 * 64;
        let mut bytes = [0u8; PAGE_SIZE];
        let mut sink = Sink::new(&mut bytes[..]);
        branch.encode(&mut sink);
        let len = branch.encoded_len();
        let branch = (&bytes[..len]).to_vec();

        info!("Finishing GetOpening request {}", uuid);
        Ok(Response::new(GetOpeningResponse { branch }))
    }

    async fn get_stake(
        &self,
        request: Request<GetStakeRequest>,
    ) -> Result<Response<GetStakeResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received GetStake request {}", uuid);

        const ERR: Error = Error::Serialization(dusk_bytes::Error::InvalidData);

        let mut bytes = [0u8; PublicKey::SIZE];

        let pk = request.get_ref().pk.as_slice();

        if pk.len() < PublicKey::SIZE {
            return Err(ERR.into());
        }

        (&mut bytes[..PublicKey::SIZE]).copy_from_slice(&pk[..PublicKey::SIZE]);

        let pk = PublicKey::from_bytes(&bytes).map_err(|_| ERR)?;

        let stake = self.state(uuid)?.fetch_stake(&pk)?;
        let amount = stake
            .amount()
            .copied()
            .map(|(value, eligibility)| Amount { value, eligibility });

        info!("Finishing GetStake request {}", uuid);
        Ok(Response::new(GetStakeResponse {
            amount,
            reward: stake.reward(),
            counter: stake.counter(),
        }))
    }

    async fn find_existing_nullifiers(
        &self,
        request: Request<FindExistingNullifiersRequest>,
    ) -> Result<Response<FindExistingNullifiersResponse>, Status> {
        let uuid = Uuid::new_v4();
        info!("Received FindExistingNullifiers request {}", uuid);

        let nullifiers = &request.get_ref().nullifiers;

        let nullifiers = nullifiers
            .iter()
            .map(|n| BlsScalar::from_slice(n).map_err(Error::Serialization))
            .collect::<Result<Vec<_>, _>>()?;

        let nullifiers = self
            .state(uuid)?
            .transfer_contract()?
            .find_existing_nullifiers(&nullifiers)
            .map_err(|_| {
                Error::Serialization(dusk_bytes::Error::InvalidData)
            })?;

        let nullifiers =
            nullifiers.iter().map(|n| n.to_bytes().to_vec()).collect();

        info!("Finishing FindExistingNullifiers request {}", uuid);
        Ok(Response::new(FindExistingNullifiersResponse { nullifiers }))
    }
}
