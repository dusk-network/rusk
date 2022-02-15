// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::services::prover::RuskProver;
use crate::{Result, Rusk, RuskState};

use crate::services::TX_VERSION;
use canonical::{Canon, Sink};
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::ViewKey;
use dusk_wallet_core::Transaction;
use phoenix_core::Note;
use rusk_abi::hash::Hasher;
use rusk_vm::GasMeter;
use tonic::{Request, Response, Status};
use tracing::info;

pub use super::rusk_proto::state_server::{State, StateServer};
pub use super::rusk_proto::{
    EchoRequest, EchoResponse, ExecuteStateTransitionRequest,
    ExecuteStateTransitionResponse,
    ExecutedTransaction as ExecutedTransactionProto,
    FindExistingNullifiersRequest, FindExistingNullifiersResponse,
    GetAnchorRequest, GetAnchorResponse, GetNotesOwnedByRequest,
    GetNotesOwnedByResponse, GetOpeningRequest, GetOpeningResponse,
    GetProvisionersRequest, GetProvisionersResponse, GetStakeRequest,
    GetStakeResponse, GetStateRootRequest, GetStateRootResponse,
    PreverifyRequest, PreverifyResponse, Provisioner, RevertRequest,
    RevertResponse, Stake as StakeProto, StateTransitionRequest,
    StateTransitionResponse, Transaction as TransactionProto,
    VerifyStateTransitionRequest, VerifyStateTransitionResponse,
};

pub(crate) type SpentTransaction = (Transaction, GasMeter);

use super::TX_TYPE_COINBASE;

/// Partition transactions into transfer and coinbase notes.
fn extract_coinbase(
    txs: Vec<TransactionProto>,
) -> Result<(Vec<TransactionProto>, (Note, Note)), Status> {
    let (coinbase_txs, transfer_txs): (Vec<_>, Vec<_>) = txs
        .into_iter()
        .partition(|tx| tx.r#type == TX_TYPE_COINBASE);

    let coinbases = coinbase_txs.len();
    if coinbases != 1 {
        return Err(Status::invalid_argument(format!(
            "Expected 1 coinbase, found {}",
            coinbases
        )));
    }

    let mut reader = &coinbase_txs[0].payload[..];
    let dusk_note =
        Note::from_reader(&mut reader).map_err(Error::Serialization)?;
    let generator_note =
        Note::from_reader(&mut reader).map_err(Error::Serialization)?;

    Ok((transfer_txs, (dusk_note, generator_note)))
}

impl Rusk {
    fn verify(&self, tx: &Transaction) -> Result<(), Status> {
        if self.state()?.any_nullifier_exists(tx.inputs())? {
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
        request: Request<StateTransitionRequest>,
    ) -> Result<(Response<StateTransitionResponse>, RuskState), Status> {
        let request = request.into_inner();

        let mut state = self.state()?;
        let mut block_gas_meter = GasMeter::with_limit(request.block_gas_limit);

        let (transfer_txs, coinbase) = extract_coinbase(request.txs)?;

        let txs = self.execute_transactions(
            &mut state,
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

    fn execute_transactions<T>(
        &self,
        state: &mut RuskState,
        block_gas_meter: &mut GasMeter,
        block_height: u64,
        txs: &[TransactionProto],
    ) -> Vec<T>
    where
        T: From<SpentTransaction>,
    {
        txs.iter()
            .map(|tx| Transaction::from_slice(&tx.payload))
            .filter_map(|tx| tx.ok())
            .map(|tx| {
                let mut gas_meter = GasMeter::with_limit(tx.fee().gas_limit);

                // We do not care if the transaction fails or succeeds here
                let _ = state.execute::<()>(
                    block_height,
                    tx.clone(),
                    &mut gas_meter,
                );

                (tx, gas_meter)
            })
            .take_while(|(_, gas_meter)| {
                block_gas_meter.charge(gas_meter.spent()).is_ok()
            })
            .map(|tx_spent| tx_spent.into())
            .collect()
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
        info!("Received Preverify request");

        let request = request.into_inner();

        let tx_proto = request.tx.ok_or_else(|| {
            Status::invalid_argument("Transaction is required")
        })?;

        let tx = Transaction::from_slice(&tx_proto.payload)
            .map_err(Error::Serialization)?;

        let tx_hash = tx.hash();

        self.verify(&tx)?;

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

        let mut state = self.state()?;

        let request = request.into_inner();

        let mut block_gas_meter = GasMeter::with_limit(request.block_gas_limit);

        let mut txs = self.execute_transactions(
            &mut state,
            &mut block_gas_meter,
            request.block_height,
            &request.txs,
        );

        // Mint coinbase notes and add a coinbase transaction to block
        let (dusk_note, generator_note) = state.mint(
            request.block_height,
            block_gas_meter.spent(),
            self.generator.as_ref(),
        )?;

        let mut payload = Vec::with_capacity(2 * Note::SIZE);

        payload.extend(dusk_note.to_bytes());
        payload.extend(generator_note.to_bytes());

        let tx_hash = Hasher::digest(&payload).to_bytes().to_vec();

        txs.push(ExecutedTransactionProto {
            tx: Some(TransactionProto {
                version: TX_VERSION,
                r#type: TX_TYPE_COINBASE,
                payload,
            }),
            tx_hash,
            gas_spent: 0, // coinbase transactions never cost anything
        });

        // Compute the new state root resulting from the state changes
        let state_root = state.root().to_vec();

        let success = true;

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
        let mut block_gas_meter = GasMeter::with_limit(request.block_gas_limit);

        let (transfer_txs, coinbase) = extract_coinbase(request.txs)?;

        let success = transfer_txs
            .iter()
            .map(|tx| Transaction::from_slice(&tx.payload))
            .all(|tx| match tx {
                Ok(tx) if self.verify(&tx).is_err() => false,
                Ok(tx) => {
                    let block_height = request.block_height;
                    let mut gas_meter =
                        GasMeter::with_limit(tx.fee().gas_limit);

                    let _ =
                        state.execute::<()>(block_height, tx, &mut gas_meter);

                    block_gas_meter.charge(gas_meter.spent()).is_ok()
                }
                Err(_) => false,
            });

        if !success {
            return Err(Status::invalid_argument("Invalid transactions block"));
        }

        let success = state
            .push_coinbase(
                request.block_height,
                block_gas_meter.spent(),
                coinbase,
            )
            .is_ok();

        if !success {
            return Err(Status::invalid_argument(
                "Invalid coinbase in the block",
            ));
        }

        Ok(Response::new(VerifyStateTransitionResponse {}))
    }

    async fn accept(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        info!("Received Accept request");

        let (response, mut state) = self.accept_transactions(request)?;

        state.accept();
        self.persist(&mut state)?;

        Ok(response)
    }

    async fn finalize(
        &self,
        request: Request<StateTransitionRequest>,
    ) -> Result<Response<StateTransitionResponse>, Status> {
        info!("Received Finalize request");

        let (response, mut state) = self.accept_transactions(request)?;

        state.finalize();
        self.persist(&mut state)?;

        Ok(response)
    }

    async fn revert(
        &self,
        _request: Request<RevertRequest>,
    ) -> Result<Response<RevertResponse>, Status> {
        info!("Received Revert request");

        let mut state = self.state()?;

        state.revert();
        self.persist(&mut state)?;

        let state_root = state.root().to_vec();
        Ok(Response::new(RevertResponse { state_root }))
    }

    async fn get_provisioners(
        &self,
        _request: Request<GetProvisionersRequest>,
    ) -> Result<Response<GetProvisionersResponse>, Status> {
        info!("Received GetProvisioners request");

        let state = self.state()?;
        let provisioners = state
            .get_provisioners()?
            .into_iter()
            .map(|(key, stake)| {
                let raw_public_key_bls = key.to_raw_bytes().to_vec();
                let public_key_bls = key.to_bytes().to_vec();

                let stake = StakeProto {
                    value: stake.value(),
                    created_at: stake.created_at(),
                    eligibility: stake.eligibility(),
                };

                Provisioner {
                    raw_public_key_bls,
                    public_key_bls,
                    stakes: vec![stake],
                }
            })
            .collect();

        Ok(Response::new(GetProvisionersResponse { provisioners }))
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
        let block_height = request.get_ref().height;

        let state = self.state()?;
        let notes = state
            .fetch_notes(block_height, &vk)?
            .iter()
            .map(|note| note.to_bytes().to_vec())
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

        const ERR: Error = Error::Serialization(dusk_bytes::Error::InvalidData);

        let mut bytes = [0u8; PublicKey::SIZE];

        let pk = request.get_ref().pk.as_slice();

        if pk.len() < PublicKey::SIZE {
            return Err(ERR.into());
        }

        (&mut bytes[..PublicKey::SIZE]).copy_from_slice(&pk[..PublicKey::SIZE]);

        let pk = PublicKey::from_bytes(&bytes).map_err(|_| ERR)?;

        let stake = self.state()?.fetch_stake(&pk)?;

        Ok(Response::new(GetStakeResponse {
            value: stake.value(),
            created_at: stake.created_at(),
            eligibility: stake.eligibility(),
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

        let nullifiers = self
            .state()?
            .transfer_contract()?
            .find_existing_nullifiers(&nullifiers)
            .map_err(|_| {
                Error::Serialization(dusk_bytes::Error::InvalidData)
            })?;

        let nullifiers =
            nullifiers.iter().map(|n| n.to_bytes().to_vec()).collect();

        Ok(Response::new(FindExistingNullifiersResponse { nullifiers }))
    }
}
