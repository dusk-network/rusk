use bytehash::Blake2b;
use dusk_abi::H256;
use kelvin::Root;
use phoenix::{rpc, Scalar, Transaction};
use phoenix_abi::{
    types::{MAX_NOTES_PER_TRANSACTION, MAX_NULLIFIERS_PER_TRANSACTION},
    Note as ABINote, Nullifier as ABINullifier,
};
use rusk_vm::{Contract, GasMeter, NetworkState, Schedule, StandardABI};
use tracing::trace;
use transfer::transfer;

pub struct Rusk {
    contract_id: H256,
}

impl Default for Rusk {
    fn default() -> Self {
        let schedule = Schedule::default();
        let contract = Contract::new(include_bytes!("../../rusk-vm/tests/contracts/transfer/wasm/target/wasm32-unknown-unknown/release/transfer.wasm"), &schedule).unwrap();

        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        let contract_id = network.deploy(contract).unwrap();
        Self {
            contract_id: contract_id,
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
                Transaction::try_from_rpc_transaction("/tmp/rusk-vm-demo", t).map_err(
                    |e| tonic::Status::invalid_argument(e.to_string()),
                )?,
            );
        }

        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        txs.iter().for_each(|tx| {
            let mut gas = GasMeter::with_limit(1_000_000_000);

            let mut nullifiers: Vec<ABINullifier> = vec![];
            let mut notes: Vec<ABINote> = vec![];

            tx.items().iter().for_each(|item| {
                if *item.nullifier().point() != Scalar::default() {
                    let nullifier =
                        ABINullifier::from(item.nullifier().clone());
                    nullifiers.push(nullifier);
                } else {
                    let note = ABINote::from(item.clone());
                    notes.push(note);
                }
            });

            // Swap to arrays
            let mut nul_arr =
                [ABINullifier::default(); MAX_NULLIFIERS_PER_TRANSACTION];
            let mut note_arr = [ABINote::default(); MAX_NOTES_PER_TRANSACTION];

            for (i, nul) in nullifiers.drain(..).enumerate() {
                nul_arr[i] = nul;
            }

            for (i, note) in notes.drain(..).enumerate() {
                note_arr[i] = note;
            }

            network
                .call_contract(
                    &self.contract_id,
                    transfer(nul_arr, note_arr),
                    &mut gas,
                )
                .unwrap();
        });

        Ok(tonic::Response::new(rpc::ValidateStateTransitionResponse {
            success: true,
        }))
    }
}
