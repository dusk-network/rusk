use dataview::Pod;
use kelvin::{Blake2b, Root};
use phoenix::{
    db, db::DbNotesIterator, rpc, utils, Error, Note, NoteGenerator,
    NoteVariant, ObfuscatedNote, PublicKey, SecretKey, Transaction,
    TransactionInput, TransactionItem, TransparentNote, ViewKey,
};
use phoenix_abi::{Input as ABIInput, Note as ABINote, Proof as ABIProof};
use rusk_vm::dusk_abi::H256;
use rusk_vm::{Contract, GasMeter, NetworkState, Schedule, StandardABI};
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::path::Path;
use tracing::trace;

fn error_to_tonic(e: Error) -> tonic::Status {
    e.into()
}

pub struct Rusk {
    transfer_id: H256,
}

// Transfer Contract args
#[repr(C)]
struct TransferCall(
    [ABIInput; ABIInput::MAX],
    [ABINote; ABINote::MAX],
    ABIProof,
);

unsafe impl Pod for TransferCall {}

impl Default for Rusk {
    fn default() -> Self {
        let schedule = Schedule::default();

        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        // TODO: For development only
        let transfer_contract = Contract::new(
            fs::read("./contracts/transfer/target/wasm32-unknown-unknown/release/transfer.wasm").unwrap().as_slice(),
            &schedule,
        )
        .unwrap();

        let transfer_id = network.deploy(transfer_contract).unwrap();

        root.set_root(&mut network).unwrap();
        Self { transfer_id }
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
        let root = Root::<_, Blake2b>::new("/tmp/rusk-state")?;
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore()?;

        let correct_txs: Vec<rpc::ContractCall> = request
            .into_inner()
            .calls
            .into_iter()
            .filter_map(|t| match t.contract_call? {
                rpc::contract_call::ContractCall::Tx(t) => {
                    let mut input_arr = [ABIInput::default(); ABIInput::MAX];
                    let mut note_arr = [ABINote::default(); ABINote::MAX];

                    for (i, input) in t.inputs.iter().enumerate() {
                        let abi_input = ABIInput::try_from(input).ok()?;
                        input_arr[i] = abi_input;
                    }

                    for (i, output) in t.outputs.iter().enumerate() {
                        let abi_note = ABINote::try_from(output).ok()?;
                        note_arr[i] = abi_note;
                    }

                    let fee_note = ABINote::try_from(t.fee.as_ref()?).ok()?;
                    note_arr[ABINote::MAX - 1] = fee_note;

                    let mut gas = GasMeter::with_limit(1_000_000_000);

                    let mut proof_buf = [0u8; ABIProof::SIZE];
                    proof_buf.copy_from_slice(&t.proof);
                    let proof = ABIProof::from_bytes(proof_buf);

                    network
                        .call_contract_operation::<TransferCall, i32>(
                            self.transfer_id,
                            1, // Transfer opcode
                            TransferCall(input_arr, note_arr, proof),
                            &mut gas,
                        )
                        .ok()?;

                    Some(rpc::ContractCall {
                        contract_call: Some(
                            rpc::contract_call::ContractCall::Tx(t),
                        ),
                    })
                }
                // TODO: add logic for handling other types of contract calls
                _ => None,
            })
            .collect();

        Ok(tonic::Response::new(rpc::ValidateStateTransitionResponse {
            successful_calls: correct_txs,
        }))
    }

    // TODO: implement
    async fn execute_state_transition(
        &self,
        _request: tonic::Request<rpc::ExecuteStateTransitionRequest>,
    ) -> Result<
        tonic::Response<rpc::ExecuteStateTransitionResponse>,
        tonic::Status,
    > {
        unimplemented!()
    }

    // TODO: implement
    async fn generate_score(
        &self,
        _request: tonic::Request<rpc::GenerateScoreRequest>,
    ) -> Result<tonic::Response<rpc::GenerateScoreResponse>, tonic::Status>
    {
        unimplemented!()
    }

    async fn generate_secret_key(
        &self,
        request: tonic::Request<rpc::GenerateSecretKeyRequest>,
    ) -> Result<tonic::Response<rpc::SecretKey>, tonic::Status> {
        trace!("Incoming generate secret key request");
        let sk = SecretKey::from(request.into_inner().b.as_slice());
        let sk = rpc::SecretKey::from(sk);
        Ok(tonic::Response::new(sk))
    }

    async fn keys(
        &self,
        request: tonic::Request<rpc::SecretKey>,
    ) -> Result<tonic::Response<rpc::KeysResponse>, tonic::Status> {
        trace!("Incoming keys request");
        let sk = request.into_inner();

        let a =
            utils::deserialize_jubjub_scalar(&sk.a.unwrap_or_default().data)
                .map_err(error_to_tonic)?;
        let b =
            utils::deserialize_jubjub_scalar(&sk.b.unwrap_or_default().data)
                .map_err(error_to_tonic)?;

        let sk = SecretKey::new(a, b);
        let vk: rpc::ViewKey = sk.view_key().into();
        let pk: rpc::PublicKey = sk.public_key().into();

        let keys = rpc::KeysResponse {
            vk: Some(vk),
            pk: Some(pk),
        };

        Ok(tonic::Response::new(keys))
    }

    async fn full_scan_owned_notes(
        &self,
        request: tonic::Request<rpc::ViewKey>,
    ) -> Result<tonic::Response<rpc::OwnedNotesResponse>, tonic::Status> {
        trace!("Incoming full scan owned notes request");
        let vk: ViewKey =
            request.into_inner().try_into().map_err(error_to_tonic)?;

        let db_path = &std::env::var("PHOENIX_DB").or_else(|_| {
            Err(tonic::Status::new(
                tonic::Code::Internal,
                "could not get db path",
            ))
        })?;

        let notes_iter: DbNotesIterator<Blake2b> =
            DbNotesIterator::try_from(Path::new(db_path).to_path_buf())?;
        let mut notes: Vec<rpc::DecryptedNote> = vec![];
        notes_iter.for_each(|note: NoteVariant| {
            if note.is_owned_by(&vk) {
                notes.push(note.rpc_decrypted_note(&vk))
            }
        });

        Ok(tonic::Response::new(rpc::OwnedNotesResponse { notes }))
    }

    async fn new_transaction(
        &self,
        request: tonic::Request<rpc::NewTransactionRequest>,
    ) -> Result<tonic::Response<rpc::Transaction>, tonic::Status> {
        trace!("Incoming new transaction request");
        let request = request.into_inner();

        // Ensure the SK exists
        let sk = request.sk.ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::InvalidArgument,
                "no secret key provided",
            )
        })?;
        // Ensure PK exists
        let pk = request.recipient.ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::InvalidArgument,
                "no secret key provided",
            )
        })?;

        let input_amount = request
            .inputs
            .iter()
            .fold(0, |acc, input| acc + input.value);
        if input_amount < request.value + request.fee {
            return Err(tonic::Status::new(
                tonic::Code::Cancelled,
                "input amount too low",
            ));
        }

        let mut tx = Transaction::default();

        let db_path = &std::env::var("PHOENIX_DB").or_else(|_| {
            Err(tonic::Status::new(
                tonic::Code::Internal,
                "could not get db path",
            ))
        })?;

        let inputs: Vec<TransactionInput> = request
            .inputs
            .into_iter()
            .map(|input| {
                // TODO: handle this error properly
                let note = match input.note_type.try_into()? {
                    rpc::NoteType::Transparent => NoteVariant::Transparent(
                        TransparentNote::try_from(input)?,
                    ),
                    rpc::NoteType::Obfuscated => NoteVariant::Obfuscated(
                        ObfuscatedNote::try_from(input)?,
                    ),
                };

                let merkle_proof =
                    db::merkle_opening(Path::new(db_path), &note)?;
                Ok(note
                    .to_transaction_input(merkle_proof, sk.clone().try_into()?))
            })
            .collect::<Result<Vec<TransactionInput>, tonic::Status>>()?;

        // TODO: when we can add more than one, turn this into a for loop
        tx.push_input(inputs[0])?;

        // Make output note
        let pk: PublicKey = pk.try_into()?;
        let (note, blinding_factor) =
            TransparentNote::output(&pk, request.value);

        tx.push_output(note.to_transaction_output(
            request.value,
            blinding_factor,
            pk,
        ))?;

        // Make change note if needed
        let change = inputs[0].value() - (request.value + request.fee);
        if change > 0 {
            let secret_key: SecretKey = sk.try_into()?;
            let pk = secret_key.public_key();
            let (note, blinding_factor) = TransparentNote::output(&pk, change);

            tx.push_output(note.to_transaction_output(
                change,
                blinding_factor,
                pk,
            ))?;
        }

        // Make fee note
        let (note, blinding_factor) = TransparentNote::output(&pk, request.fee);

        tx.set_fee(note.to_transaction_output(
            request.fee,
            blinding_factor,
            pk,
        ));

        tx.prove()?;

        Ok(tonic::Response::new(tx.try_into()?))
    }

    // TODO: implement
    async fn verify_transaction(
        &self,
        _request: tonic::Request<rpc::Transaction>,
    ) -> Result<tonic::Response<rpc::VerifyTransactionResponse>, tonic::Status>
    {
        trace!("Incoming verify transaction request");
        unimplemented!()
        /*
        Transaction::try_from_rpc_transaction(DB_PATH, request.into_inner())
            .and_then(|tx| tx.verify())
            .map(|_| tonic::Response::new(rpc::VerifyTransactionResponse {}))
            .map_err(error_to_tonic)
        */
    }
}

#[cfg(test)]
#[path = "./test_contract_transfer.rs"]
mod test_contract_transfer;
