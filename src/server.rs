use bytehash::Blake2b;
use kelvin::{ByteHash, Root};
use phoenix::{
    db, db::Db, db::DbNotesIterator, rpc, utils, zk, Error, Note,
    NoteGenerator, NoteVariant, Nullifier, ObfuscatedNote, PublicKey,
    SecretKey, Transaction, TransactionInput, TransactionItem,
    TransactionOutput, TransparentNote, ViewKey,
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
        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        let correct_txs: Vec<bool> = request
            .into_inner()
            .txs
            .iter()
            .filter_map(|t| {
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

                let fee_note =
                    ABINote::try_from(t.fee.as_ref().unwrap()).ok()?;
                note_arr[ABINote::MAX - 1] = fee_note;

                let mut gas = GasMeter::with_limit(1_000_000_000);

                let mut proof_buf = [0u8; ABIProof::SIZE];
                proof_buf.copy_from_slice(&t.proof);
                let proof = ABIProof(proof_buf);
                let call: ContractCall<bool> =
                    ContractCall::new(TransferCall::Transfer {
                        inputs: input_arr,
                        notes: note_arr,
                        proof,
                    })
                    .unwrap();
                let result =
                    network.call_contract(&self.transfer_id, call, &mut gas);
                println!("{:?}", result);
                result.ok()
            })
            .collect();
        if correct_txs.len() > 0 {
            return Ok(tonic::Response::new(
                rpc::ValidateStateTransitionResponse { success: true },
            ));
        }

        Ok(tonic::Response::new(rpc::ValidateStateTransitionResponse {
            success: false,
        }))
    }

    async fn generate_secret_key(
        &self,
        request: tonic::Request<rpc::GenerateSecretKeyRequest>,
    ) -> Result<tonic::Response<rpc::SecretKey>, tonic::Status> {
        trace!("Incoming generate secret key request");
        unimplemented!()
        /*
        let sk = SecretKey::from(request.into_inner().b.as_slice());
        let sk = rpc::SecretKey::from(sk);
        Ok(tonic::Response::new(sk))
        */
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

    async fn nullifier(
        &self,
        request: tonic::Request<rpc::NullifierRequest>,
    ) -> Result<tonic::Response<rpc::NullifierResponse>, tonic::Status> {
        trace!("Incoming nullifier request");
        let request = request.into_inner();

        let sk: SecretKey = request
            .sk
            .ok_or(Error::InvalidParameters)
            .map_err(error_to_tonic)?
            .try_into()
            .map_err(error_to_tonic)?;
        let note: NoteVariant = request
            .note
            .ok_or(Error::InvalidParameters)
            .and_then(|note| note.try_into())
            .map_err(error_to_tonic)?;

        let nullifier = note.generate_nullifier(&sk);
        let response = rpc::NullifierResponse {
            nullifier: Some(nullifier.into()),
        };

        Ok(tonic::Response::new(response))
    }

    async fn nullifier_status(
        &self,
        request: tonic::Request<rpc::NullifierStatusRequest>,
    ) -> Result<tonic::Response<rpc::NullifierStatusResponse>, tonic::Status>
    {
        trace!("Incoming nullifier status request");
        unimplemented!()
        /*
        let request = request.into_inner();

        let nullifier: Nullifier = request
            .nullifier
            .ok_or(error_to_tonic(Error::InvalidParameters))?
            .into();

        let unspent = db::fetch_nullifier(DB_PATH, &nullifier)
            .map(|r| r.is_none())
            .map_err(error_to_tonic)?;

        let response = rpc::NullifierStatusResponse { unspent };
        Ok(tonic::Response::new(response))
        */
    }

    async fn fetch_note(
        &self,
        request: tonic::Request<rpc::FetchNoteRequest>,
    ) -> Result<tonic::Response<rpc::Note>, tonic::Status> {
        trace!("Incoming fetch note request");
        unimplemented!()
        /*
        let idx: u64 = request.into_inner().pos;
        let note = db::fetch_note(&DB_PATH, idx)
            .map(|note| note.into())
            .map_err(error_to_tonic)?;

        Ok(tonic::Response::new(note))
        */
    }

    async fn decrypt_note(
        &self,
        request: tonic::Request<rpc::DecryptNoteRequest>,
    ) -> Result<tonic::Response<rpc::DecryptedNote>, tonic::Status> {
        trace!("Incoming decrypt note request");
        unimplemented!()
        /*
        let request = request.into_inner();

        let note: NoteVariant = request
            .note
            .ok_or(Error::InvalidParameters)
            .and_then(|note| note.try_into())
            .map_err(error_to_tonic)?;

        let vk: ViewKey = request
            .vk
            .ok_or(Error::InvalidParameters)
            .and_then(|vk| vk.try_into())
            .map_err(error_to_tonic)?;

        let note = note.rpc_decrypted_note(&vk);
        Ok(tonic::Response::new(note))
        */
    }

    async fn owned_notes(
        &self,
        request: tonic::Request<rpc::OwnedNotesRequest>,
    ) -> Result<tonic::Response<rpc::OwnedNotesResponse>, tonic::Status> {
        trace!("Incoming owned notes request");
        unimplemented!()
        /*
        let request = request.into_inner();

        let vk: ViewKey = request
            .vk
            .ok_or(Error::InvalidParameters)
            .and_then(|vk| vk.try_into())
            .map_err(error_to_tonic)?;

        let notes: Vec<rpc::DecryptedNote> = request
            .notes
            .into_iter()
            .try_fold(vec![], |mut notes, note| {
                let note: NoteVariant = note.try_into()?;

                if note.is_owned_by(&vk) {
                    notes.push(note.rpc_decrypted_note(&vk));
                }

                Ok(notes)
            })
            .map_err(error_to_tonic)?;

        Ok(tonic::Response::new(rpc::OwnedNotesResponse { notes }))
        */
    }

    async fn full_scan_owned_notes(
        &self,
        request: tonic::Request<rpc::ViewKey>,
    ) -> Result<tonic::Response<rpc::OwnedNotesResponse>, tonic::Status> {
        trace!("Incoming full scan owned notes request");
        let vk: ViewKey =
            request.into_inner().try_into().map_err(error_to_tonic)?;

        let root = Root::<_, Blake2b>::new(Path::new(
            &std::env::var("PHOENIX_DB").unwrap(),
        ))
        .unwrap();
        let db: Db<_> = root.restore().unwrap();

        let notes_iter: DbNotesIterator<Blake2b> = DbNotesIterator::try_from(
            Path::new(&std::env::var("PHOENIX_DB").unwrap()).to_path_buf(),
        )
        .unwrap();
        let mut notes: Vec<rpc::DecryptedNote> = vec![];
        notes_iter.for_each(|note: NoteVariant| {
            if note.is_owned_by(&vk) {
                notes.push(note.rpc_decrypted_note(&vk))
            }
        });

        Ok(tonic::Response::new(rpc::OwnedNotesResponse { notes }))
    }

    async fn new_transaction_input(
        &self,
        request: tonic::Request<rpc::NewTransactionInputRequest>,
    ) -> Result<tonic::Response<rpc::TransactionInput>, tonic::Status> {
        trace!("Incoming new transaction input request");
        unimplemented!()
        /*
        let request = request.into_inner();

        let idx: u64 = request.pos;

        let sk: SecretKey = request
            .sk
            .ok_or(Error::InvalidParameters)
            .map_err(error_to_tonic)?
            .try_into()
            .map_err(error_to_tonic)?;

        let txi = db::fetch_note(&DB_PATH, idx)
            .map_err(error_to_tonic)?
            .to_transaction_input(sk);
        let txi: rpc::TransactionInput = txi.into();

        Ok(tonic::Response::new(txi))
        */
    }

    async fn new_transaction_output(
        &self,
        request: tonic::Request<rpc::NewTransactionOutputRequest>,
    ) -> Result<tonic::Response<rpc::TransactionOutput>, tonic::Status> {
        trace!("Incoming new transaction output request");
        unimplemented!()
        /*
        let request = request.into_inner();

        let pk: PublicKey = request
            .pk
            .ok_or(Error::InvalidParameters)
            .and_then(|pk| pk.try_into())
            .map_err(error_to_tonic)?;

        let note_type: rpc::NoteType =
            request.note_type.try_into().map_err(error_to_tonic)?;

        let txo = match note_type {
            NoteType::Transparent => {
                let (note, blinding_factor) =
                    TransparentNote::output(&pk, request.value);
                note.to_transaction_output(request.value, blinding_factor, pk)
            }
            NoteType::Obfuscated => {
                let (note, blinding_factor) =
                    ObfuscatedNote::output(&pk, request.value);
                note.to_transaction_output(request.value, blinding_factor, pk)
            }
        };

        let txo: rpc::TransactionOutput = txo.into();
        Ok(tonic::Response::new(txo))
        */
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

        let inputs: Vec<TransactionInput> = request
            .inputs
            .into_iter()
            .map(|input| {
                // TODO: handle this error properly
                let note = match input.note_type.try_into().unwrap() {
                    rpc::NoteType::Transparent => NoteVariant::Transparent(
                        TransparentNote::try_from(input).unwrap(),
                    ),
                    rpc::NoteType::Obfuscated => NoteVariant::Obfuscated(
                        ObfuscatedNote::try_from(input).unwrap(),
                    ),
                };

                let merkle_proof = db::merkle_opening(
                    Path::new(&std::env::var("PHOENIX_DB").unwrap()),
                    &note,
                )
                .unwrap();
                note.to_transaction_input(
                    merkle_proof,
                    sk.clone().try_into().unwrap(),
                )
            })
            .collect::<Vec<TransactionInput>>();

        // TODO: when we can add more than one, turn this into a for loop
        tx.push_input(inputs[0]).unwrap();

        // Make output note
        let pk: PublicKey = pk.try_into().unwrap();
        let (note, blinding_factor) =
            TransparentNote::output(&pk, request.value);

        tx.push_output(note.to_transaction_output(
            request.value,
            blinding_factor,
            pk,
        ))
        .unwrap();

        // Make change note if needed
        let change = inputs[0].value() - (request.value + request.fee);
        if change > 0 {
            let (note, blinding_factor) = TransparentNote::output(&pk, change);

            tx.push_output(note.to_transaction_output(
                change,
                blinding_factor,
                pk,
            ))
            .unwrap();
        }

        // Make fee note
        let (note, blinding_factor) = TransparentNote::output(&pk, request.fee);

        tx.set_fee(note.to_transaction_output(
            request.fee,
            blinding_factor,
            pk,
        ));

        tx.prove().unwrap();

        Ok(tonic::Response::new(tx.try_into().unwrap()))
    }

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

    async fn verify_transaction_root(
        &self,
        _request: tonic::Request<rpc::VerifyTransactionRootRequest>,
    ) -> Result<
        tonic::Response<rpc::VerifyTransactionRootResponse>,
        tonic::Status,
    > {
        trace!("Incoming verify transaction root request");
        unimplemented!()
    }

    async fn store_transactions(
        &self,
        request: tonic::Request<rpc::StoreTransactionsRequest>,
    ) -> Result<tonic::Response<rpc::StoreTransactionsResponse>, tonic::Status>
    {
        trace!("Incoming store transactions request");
        unimplemented!()
        /*
        let request = request.into_inner();
        let mut transactions = vec![];

        for tx in request.transactions {
            let tx = Transaction::try_from_rpc_transaction(DB_PATH, tx)
                .map_err(error_to_tonic)?;

            transactions.push(tx);
        }

        for tx in &mut transactions {
            tx.verify().map_err(error_to_tonic)?;
        }

        let notes: Vec<rpc::Note> =
            db::store_bulk_transactions(DB_PATH, transactions.as_slice())
                .map_err(error_to_tonic)?
                .iter()
                .try_fold(vec![], |mut v, idx| {
                    v.push(db::fetch_note(DB_PATH, *idx)?.into());
                    Ok(v)
                })
                .map_err(error_to_tonic)?;

        // let root: rpc::Scalar = db.root().into();
        let root = Some(rpc::Scalar::default());

        let response = rpc::StoreTransactionsResponse { notes, root };
        Ok(tonic::Response::new(response))
        */
    }

    async fn set_fee_pk(
        &self,
        request: tonic::Request<rpc::SetFeePkRequest>,
    ) -> Result<tonic::Response<rpc::Transaction>, tonic::Status> {
        trace!("Incoming set fee pk request");
        unimplemented!()
        /*
        let request = request.into_inner();

        let transaction = request.transaction.unwrap_or_default();
        let mut transaction =
            Transaction::try_from_rpc_transaction(DB_PATH, transaction)
                .map_err(error_to_tonic)?;

        let pk: PublicKey = request
            .pk
            .unwrap_or_default()
            .try_into()
            .map_err(error_to_tonic)?;

        transaction.set_fee_pk(pk);

        let tx = transaction.try_into().map_err(error_to_tonic)?;
        Ok(tonic::Response::new(tx))
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
        let sk_str = "b9e2d256378bd34648eb802a497977b2a14d5aae3826866baac570a4a7a1360a4c1722a9d8126e5654c5411bef959b18e58d4b4f88b07b8ab8bd42ec67c90c0b";
        let decoded = hex::decode(sk_str).unwrap();
        let mut a_bytes = [0u8; 32];
        a_bytes.copy_from_slice(&decoded[0..32]);
        let mut b_bytes = [0u8; 32];
        b_bytes.copy_from_slice(&decoded[32..64]);
        let a = utils::deserialize_jubjub_scalar(&a_bytes).unwrap();
        let b = utils::deserialize_jubjub_scalar(&b_bytes).unwrap();
        let sk = SecretKey::new(a, b);
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
        let response =
            client::validate_state_transition(vec![tx]).await.unwrap();

        println!("{:?}", response);

        // Clean up DB
        std::fs::remove_dir_all(std::path::Path::new(
            &std::env::var("PHOENIX_DB").unwrap(),
        ))
        .unwrap();
    }
}
