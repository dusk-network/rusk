use bytehash::Blake2b;
use kelvin::Root;
use phoenix::{
    db, db::DbNotesIterator, rpc, utils, Error, Note, NoteGenerator,
    NoteVariant, ObfuscatedNote, PublicKey, SecretKey, Transaction,
    TransactionInput, TransactionItem, TransactionOutput, TransparentNote,
    ViewKey,
};
use phoenix_abi::{Input as ABIInput, Note as ABINote, Proof as ABIProof};
use rusk_vm::dusk_abi::{ContractCall, TransferCall, H256};
use rusk_vm::{Contract, GasMeter, NetworkState, Schedule, StandardABI};
use std::convert::{TryFrom, TryInto};
use std::path::Path;
use tracing::trace;

fn error_to_tonic(e: Error) -> tonic::Status {
    e.into()
}

pub struct Rusk {
    transfer_id: H256,
}

impl Default for Rusk {
    fn default() -> Self {
        let schedule = Schedule::default();

        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        let transfer_contract= Contract::new(include_bytes!("../../rusk-vm/tests/contracts/transfer/wasm/target/wasm32-unknown-unknown/release/transfer.wasm"), &schedule).unwrap();

        let transfer_id = network.deploy(transfer_contract).unwrap();

        root.set_root(&mut network).unwrap();
        Self {
            transfer_id: transfer_id,
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
                    let call: ContractCall<bool> =
                        ContractCall::new(TransferCall::Transfer {
                            inputs: input_arr,
                            notes: note_arr,
                            proof,
                        })
                        .ok()?;
                    network
                        .call_contract(&self.transfer_id, call, &mut gas)
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
        request: tonic::Request<rpc::ExecuteStateTransitionRequest>,
    ) -> Result<
        tonic::Response<rpc::ExecuteStateTransitionResponse>,
        tonic::Status,
    > {
        unimplemented!()
    }

    // TODO: implement
    async fn generate_score(
        &self,
        request: tonic::Request<rpc::GenerateScoreRequest>,
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

        let db_path = &std::env::var("PHOENIX_DB").or(Err(
            tonic::Status::new(tonic::Code::Internal, "could not get db path"),
        ))?;

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
        let sk = request.sk.ok_or(tonic::Status::new(
            tonic::Code::InvalidArgument,
            "no secret key provided",
        ))?;
        // Ensure PK exists
        let pk = request.recipient.ok_or(tonic::Status::new(
            tonic::Code::InvalidArgument,
            "no secret key provided",
        ))?;

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

        let db_path = &std::env::var("PHOENIX_DB").or(Err(
            tonic::Status::new(tonic::Code::Internal, "could not get db path"),
        ))?;

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
        let output: TransactionOutput = match request.obfuscated {
            true => {
                let (note, blinding_factor) =
                    ObfuscatedNote::output(&pk, request.value);
                note.to_transaction_output(request.value, blinding_factor, pk)
            }
            false => {
                let (note, blinding_factor) =
                    TransparentNote::output(&pk, request.value);
                note.to_transaction_output(request.value, blinding_factor, pk)
            }
        };

        tx.push_output(output)?;

        // Make change note if needed
        let change = inputs[0].value() - (request.value + request.fee);
        if change > 0 {
            let secret_key: SecretKey = sk.clone().try_into()?;
            let pk = secret_key.public_key();
            let output: TransactionOutput = match request.obfuscated {
                true => {
                    let (note, blinding_factor) =
                        ObfuscatedNote::output(&pk, change);
                    note.to_transaction_output(change, blinding_factor, pk)
                }
                false => {
                    let (note, blinding_factor) =
                        TransparentNote::output(&pk, change);
                    note.to_transaction_output(change, blinding_factor, pk)
                }
            };

            tx.push_output(output)?;
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
        request: tonic::Request<rpc::Transaction>,
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
mod tests {
    use super::*;
    use crate::client;
    use phoenix::{
        rpc::rusk_server::RuskServer, utils, zk, PublicKey, SecretKey,
    };
    use tonic::transport::Server;

    #[tokio::test(threaded_scheduler)]
    async fn test_transfer() {
        // Set DB_PATH
        let mut db_path = std::env::temp_dir();
        db_path.push("phoenix-db");
        std::env::set_var("PHOENIX_DB", db_path.into_os_string());

        // Mandatory Phoenix setup
        utils::init();
        zk::init();

        let srv = RuskServer::new(Rusk::default());
        let addr = "0.0.0.0:8080";

        tokio::spawn(async move {
            Server::builder()
                .add_service(srv)
                .serve(addr.parse().unwrap())
                .await
        });

        // TODO: maybe find a less hacky way to let the server get up and running
        std::thread::sleep(std::time::Duration::from_millis(1000));

        // First, credit the sender with a note, so that he can create a transaction from it
        let sk = SecretKey::default();
        let pk = sk.public_key();

        let mut tx = Transaction::default();
        let value = 100_000_000;
        let (note, blinding_factor) = TransparentNote::output(&pk, value);
        tx.push_output(note.to_transaction_output(value, blinding_factor, pk))
            .unwrap();
        db::store(
            std::path::Path::new(&std::env::var("PHOENIX_DB").unwrap()),
            &tx,
        )
        .unwrap();

        // Now, let's make a transaction
        let recipient = PublicKey::default();
        let tx = client::create_transaction(
            sk,
            100_000 as u64,
            100 as u64,
            recipient.into(),
        )
        .await
        .unwrap();

        // And execute it on the VM
        let response = client::validate_state_transition(tx).await.unwrap();

        println!("{:?}", response);

        // Clean up DB
        std::fs::remove_dir_all(std::path::Path::new(
            &std::env::var("PHOENIX_DB").unwrap(),
        ))
        .unwrap();
    }
}
