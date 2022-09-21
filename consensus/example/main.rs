use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

use consensus::commons::RoundUpdate;
use consensus::consensus::Consensus;
use consensus::messages::Message;
use consensus::user::provisioners::{Provisioners, DUSK};
use tokio::sync::mpsc::{Receiver, Sender};

use consensus::util::pubkey::PublicKey;
use tokio::time;

fn generate_provisioners(n: usize) -> Provisioners {
    let mut p = Provisioners::new();
    for i in 0..n {
        let stake_value = 1000 * (i as u64) * DUSK;
        p.add_member_with_value(PublicKey::from_sk_seed_u64(i as u64), stake_value);
    }
    p
}

async fn perform_basic_run() {
    let mut all_to_inbound = vec![];

    let (sender_bridge, mut recv_bridge) = mpsc::channel::<Message>(1000);

    // Initialize N dummy provisioners
    let mocked = generate_provisioners(3);

    let provisioners = mocked.clone();
    for p in mocked.into_iter() {
        let (to_inbound, inbound_msgs) = mpsc::channel::<Message>(10);
        let (outbound_msgs, mut from_outbound) = mpsc::channel::<Message>(10);

        // Spawn a node which simulates a provisioner running its own consensus instance.
        spawn_node(p.0, provisioners.clone(), inbound_msgs, outbound_msgs);

        // Bridge all so that provisioners can exchange messages in a single-process setup.
        all_to_inbound.push(to_inbound.clone());

        let bridge = sender_bridge.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = from_outbound.recv().await {
                    let _ = bridge.send(msg.clone()).await;
                }
            }
        });
    }

    // clone bridge-ed messages to all provisioners.
    tokio::spawn(async move {
        loop {
            if let Some(msg) = recv_bridge.recv().await {
                for to_inbound in all_to_inbound.iter() {
                    let _ = to_inbound.send(msg.clone()).await;
                }
            }
        }
    });

    time::sleep(Duration::from_secs(120)).await;
}

/// spawn_node runs a separate thread-pool (tokio::runtime) that drives a single instance of consensus.
fn spawn_node(
    pubkey_bls: PublicKey,
    p: Provisioners,
    inbound_msgs: Receiver<Message>,
    outbound_msgs: Sender<Message>,
) {
    let _ = thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(3)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut c = Consensus::new(inbound_msgs, outbound_msgs);

                // Run consensus for 1 round
                c.reset_state_machine();
                c.spin(RoundUpdate::new(0, pubkey_bls), p.clone()).await;
            });
    });
}

fn main() {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(perform_basic_run());
}
