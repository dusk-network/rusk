use bytehash::Blake2b;
use dusk_abi::{ContractCall, FeeCall, Provisioners, Signature, H256};
use kelvin::Root;
use phoenix::{
    rpc, Nullifier, PublicKey, Scalar, Transaction, TransactionItem,
};
use phoenix_abi::{Note as ABINote, Nullifier as ABINullifier};
use rusk_vm::{Contract, GasMeter, NetworkState, Schedule, StandardABI};
use std::convert::TryFrom;
use tracing::trace;

pub struct Rusk {
    transfer_id: H256,
    fee_id: H256,
}

impl Default for Rusk {
    fn default() -> Self {
        let schedule = Schedule::default();
        let fee_contract= Contract::new(include_bytes!("../../rusk-vm/tests/contracts/fee/wasm/target/wasm32-unknown-unknown/release/fee.wasm"), &schedule).unwrap();

        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        let fee_id = network.deploy(fee_contract).unwrap();

        let transfer_contract= Contract::new(include_bytes!("../../rusk-vm/tests/contracts/transfer/wasm/target/wasm32-unknown-unknown/release/transfer.wasm"), &schedule).unwrap();

        let transfer_id = network.deploy(transfer_contract).unwrap();

        root.set_root(&mut network).unwrap();
        Self {
            fee_id: fee_id,
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

    async fn distribute(
        &self,
        request: tonic::Request<rpc::DistributeRequest>,
    ) -> Result<tonic::Response<rpc::DistributeResponse>, tonic::Status> {
        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        let mut gas = GasMeter::with_limit(1_000_000_000);
        let req = request.into_inner();

        let pk = PublicKey::try_from(req.clone().pk.unwrap()).unwrap();
        let a_g_bytes = pk.a_g.compress().to_bytes();
        let b_g_bytes = pk.b_g.compress().to_bytes();

        let mut pk_buf = [0u8; 64];
        for i in 0..32 {
            pk_buf[i] = a_g_bytes[i];
        }

        for i in 0..32 {
            pk_buf[i + 32] = b_g_bytes[i];
        }

        let mut provisioners = Provisioners::default();

        for i in 0..32 {
            provisioners.0[i] = req.clone().addresses[0].address[i];
        }

        let call: ContractCall<()> = ContractCall::new(FeeCall::Distribute {
            total_reward: req.clone().total_reward,
            addresses: provisioners,
            pk: pk_buf.into(),
        })
        .unwrap();

        println!("distributing rewards..");

        network.call_contract(&self.fee_id, call, &mut gas).unwrap();

        root.set_root(&mut network).unwrap();

        println!("done");
        Ok(tonic::Response::new(rpc::DistributeResponse {
            success: true,
        }))
    }

    async fn withdraw(
        &self,
        request: tonic::Request<rpc::WithdrawRequest>,
    ) -> Result<tonic::Response<rpc::WithdrawResponse>, tonic::Status> {
        let mut root = Root::<_, Blake2b>::new("/tmp/rusk-state").unwrap();
        let mut network: NetworkState<StandardABI<_>, Blake2b> =
            root.restore().unwrap();

        let mut gas = GasMeter::with_limit(1_000_000_000);
        let mut address = [0u8; 32];
        let req = request.into_inner();
        address.copy_from_slice(&req.address);

        let pk = PublicKey::try_from(req.clone().pk.unwrap()).unwrap();
        let a_g_bytes = pk.a_g.compress().to_bytes();
        let b_g_bytes = pk.b_g.compress().to_bytes();

        let mut pk_buf = [0u8; 64];
        for i in 0..32 {
            pk_buf[i] = a_g_bytes[i];
        }

        for i in 0..32 {
            pk_buf[i + 32] = b_g_bytes[i];
        }

        let call: ContractCall<()> = ContractCall::new(FeeCall::Withdraw {
            sig: Signature::from_slice(&req.signature),
            address: address,
            value: req.clone().value,
            pk: pk_buf.into(),
        })
        .unwrap();

        println!("withdrawing provisioner reward");

        network.call_contract(&self.fee_id, call, &mut gas).unwrap();

        root.set_root(&mut network).unwrap();

        println!("done");
        Ok(tonic::Response::new(rpc::WithdrawResponse {
            success: true,
        }))
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
                let mut item = TransactionItem::default();
                item.set_nullifier(Nullifier::try_from(nul).unwrap());
                tx.push(item);
            }

            for note in t.clone().outputs {
                let item = TransactionItem::try_from(note).unwrap();
                tx.push(item);
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

            tx.items().iter().for_each(|item| {
                if *item.nullifier().point() != *Nullifier::default().point() {
                    let nullifier =
                        ABINullifier::from(item.nullifier().clone());
                    nullifiers.push(nullifier);
                } else {
                    let note = ABINote::from(item.clone());
                    notes.push(note);
                }
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
