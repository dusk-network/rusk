use bytehash::Blake2b;
use kelvin::Root;
use phoenix::{
    rpc, zk, Nullifier, Transaction, TransactionInput, TransactionOutput,
};
use phoenix_abi::{
    Note as ABINote, Nullifier as ABINullifier, Proof as ABIProof,
};
use rusk_vm::dusk_abi::{ContractCall, H256};
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
        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        let correct_txs = request.into_inner().txs.iter().filter_map(|t| {
            let mut nul_arr = [ABINullifier::default(); ABINullifier::MAX];
            let mut note_arr = [ABINote::default(); ABINote::MAX];

            for (i, nul) in t.nullifiers.iter().enumerate() {
                let abi_nullifier = ABINullifier::try_from(nul).ok()?;
                nul_arr[i] = abi_nullifier;
            }

            for (i, output) in t.outputs.iter().enumerate() {
                let abi_note = ABINote::try_from(output).ok()?;
                note_arr[i] = abi_note;
            }

            let mut gas = GasMeter::with_limit(1_000_000_000);

            let mut proof_buf = [0u8; ABIProof::SIZE];
            proof_buf.copy_from_slice(&t.proof);
            let proof = ABIProof(proof_buf);
            let call: ContractCall<(
                [ABINullifier; ABINullifier::MAX],
                [ABINote; ABINote::MAX],
                ABIProof,
            )> = ContractCall::new((nul_arr, note_arr, proof)).unwrap();
            network
                .call_contract(&self.transfer_id, call, &mut gas)
                .ok()
        });

        Ok(tonic::Response::new(rpc::ValidateStateTransitionResponse {
            success: true,
        }))
    }
}
