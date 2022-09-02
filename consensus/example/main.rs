use hex::FromHex;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::trace;

use consensus::commons::RoundUpdate;
use consensus::consensus::Consensus;
use consensus::messages::Message;
use consensus::user::provisioners::{Provisioners, PublicKey, DUSK};
use tokio::task::JoinHandle;
use tokio::time;

// Message producer feeds Consensus steps with empty messages.
fn spawn_message_producer(
    inbound_msgs: mpsc::Sender<Message>,
) -> JoinHandle<u8> {
    tokio::spawn(async move {
        loop {
            return 0;

            trace!("sending new block message");
            let _ = inbound_msgs.send(Message::default()).await;

            trace!("sending first reduction message");
            let _ = inbound_msgs.send(Message::default()).await;

            trace!("sending second reduction message");
            let _ = inbound_msgs.send(Message::default()).await;

            
        }
    })
}

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
    // Initialize message sources that feeds Consensus.
    let mocked = read_provisioners();
    let provisioners = mocked.clone();
    for p in mocked.into_iter() {
        spawn_node(p.0, provisioners.clone());
    }

    time::sleep(Duration::from_secs(120)).await;
}

fn spawn_node(pubkey_bls: PublicKey, p: Provisioners) {
    tokio::spawn(async move {
        let (tx, rx) = mpsc::channel::<Message>(100);

        let producer = spawn_message_producer(tx);

        let mut c = Consensus::new(rx);
        let n = 5;
        // Run consensus for N rounds
        for r in 0..n {
            c.reset_state_machine();
            c.spin(RoundUpdate::new(r, pubkey_bls), p.clone()).await;
        }

        producer.abort();
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
