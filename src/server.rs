use bytehash::Blake2b;
use dusk_abi::{ContractCall, Provisioners, Signature, H256};
use kelvin::Root;
use phoenix::{
    rpc, Nullifier, Transaction, TransactionInput, TransactionOutput,
};
use phoenix_abi::{Note as ABINote, Nullifier as ABINullifier};
use rusk_vm::{Contract, GasMeter, NetworkState, Schedule, StandardABI};
use std::convert::TryFrom;
use tracing::trace;

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
        let mut txs = vec![];
        for t in request.into_inner().txs {
            let mut tx = Transaction::default();

            for nul in t.clone().nullifiers {
                let mut item = TransactionInput::default();
                item.nullifier = Nullifier::from(nul);
                tx.push_input(item).unwrap();
            }

            for note in t.clone().outputs {
                let item = TransactionOutput::try_from(note).unwrap();
                tx.push_output(item).unwrap();
            }

            txs.push(tx);
        }

        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        txs.iter().for_each(|tx| {
            let mut gas = GasMeter::with_limit(1_000_000_000);

            let mut nullifiers: Vec<ABINullifier> = vec![];
            let mut notes: Vec<ABINote> = vec![];

            tx.inputs().iter().for_each(|item| {
                let nullifier = ABINullifier::from(item.nullifier.clone());
                nullifiers.push(nullifier);
            });

            tx.outputs().iter().for_each(|item| {
                let note = ABINote::from(item.clone());
                notes.push(note);
            });

            let mut nul_arr = [ABINullifier::default(); ABINullifier::MAX];
            let mut note_arr = [ABINote::default(); ABINote::MAX];

            for (i, nul) in nullifiers.drain(..).enumerate() {
                nul_arr[i] = nul;
            }

            for (i, note) in notes.drain(..).enumerate() {
                note_arr[i] = note;
            }

            println!("calling contract");
            let call: ContractCall<(
                [ABINullifier; ABINullifier::MAX],
                [ABINote; ABINote::MAX],
            )> = ContractCall::new((nul_arr, note_arr)).unwrap();
            network
                .call_contract(&self.transfer_id, call, &mut gas)
                .unwrap();

            println!("done");
        });

        Ok(tonic::Response::new(rpc::ValidateStateTransitionResponse {
            success: true,
        }))
    }
}
