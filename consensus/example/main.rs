use hex::FromHex;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

use consensus::commons::RoundUpdate;
use consensus::consensus::Consensus;
use consensus::messages::Message;
use consensus::user::provisioners::{Provisioners, PublicKey, DUSK};
use tokio::sync::mpsc::{Receiver, Sender};

use tokio::time;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn read_provisioners() -> Provisioners {
    // TODO: duplciated code
    let test_data = env::var("CARGO_MANIFEST_DIR").unwrap_or_default() + "/tests/provisioners.txt";

    // Create provisioners with bls keys read from an external file.
    let mut p = Provisioners::new();
    if let Ok(lines) = read_lines(test_data) {
        let mut i = 1;
        for line in lines {
            if let Ok(bls_key) = line {
                // parse hex from file line
                let key = <[u8; 96]>::from_hex(bls_key).unwrap_or([0; 96]);
                let stake_value = 1000 * i * DUSK;

                p.add_member_with_value(PublicKey::new(key), stake_value);

                i += 1;
            }
        }
    }
    p
}

async fn perform_basic_run() {
    let mut all_to_inbound = vec![];

    let (sender_bridge, mut recv_bridge) = mpsc::channel::<Message>(1000);

    // Initialize message sources that feeds Consensus.
    let mocked = read_provisioners();
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

fn spawn_node(
    pubkey_bls: PublicKey,
    p: Provisioners,
    inbound_msgs: Receiver<Message>,
    outbound_msgs: Sender<Message>,
) {
    tokio::spawn(async move {
        let mut c = Consensus::new(inbound_msgs, outbound_msgs);
        let n = 5;
        // Run consensus for N rounds
        for r in 0..n {
            c.reset_state_machine();
            c.spin(RoundUpdate::new(r, pubkey_bls), p.clone()).await;
        }
    });
}

fn main() {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(perform_basic_run());
}
