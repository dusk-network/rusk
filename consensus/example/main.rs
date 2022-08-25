use hex::FromHex;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use tokio::sync::mpsc;
use tracing::trace;

use consensus::commons::RoundUpdate;
use consensus::consensus::Consensus;
use consensus::messages::{MsgNewBlock, MsgReduction};
use consensus::user::provisioners::{Provisioners, PublicKey, DUSK};
use tokio::task::JoinHandle;

// Message producer feeds Consensus steps with empty messages.
fn spawn_message_producer(
    tx: mpsc::Sender<MsgNewBlock>,
    red1_tx: mpsc::Sender<MsgReduction>,
    red2_tx: mpsc::Sender<MsgReduction>,
) -> JoinHandle<u8> {
    tokio::spawn(async move {
        loop {
            trace!("sending new block message");
            let _ = tx.send(MsgNewBlock::default()).await;

            trace!("sending first reduction message");
            let _ = red1_tx.send(MsgReduction::default()).await;

            trace!("sending second reduction message");
            let _ = red2_tx.send(MsgReduction::default()).await;
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
    {
        // Initialize message sources that feeds Consensus.
        let (tx, rx) = mpsc::channel::<MsgNewBlock>(100);
        let (red1_tx, first_red_rx) = mpsc::channel::<MsgReduction>(100);
        let (red2_tx, sec_red_tx) = mpsc::channel::<MsgReduction>(100);

        let producer = spawn_message_producer(tx, red1_tx, red2_tx);

        let p = read_provisioners();

        let mut c = Consensus::new(rx, first_red_rx, sec_red_tx);
        let n = 5;
        // Run consensus for N rounds
        for r in 0..n {
            c.reset_state_machine();
            c.spin(RoundUpdate::new(r, PublicKey::default()), p.clone())
                .await;
        }

        producer.abort();
    }
}

fn main() {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(perform_basic_run());
}
