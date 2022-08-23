use dusk_bls12_381_sign::{PublicKey, SecretKey};
use tokio::sync::mpsc;
use tracing::trace;

use consensus::commons::RoundUpdate;
use consensus::consensus::Consensus;
use consensus::messages::{MsgNewBlock, MsgReduction};
use consensus::user::provisioners::{HashablePubKey, Provisioners};
use rand::rngs::StdRng;
use rand::SeedableRng;
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

fn gen_mocked_provisioners() -> Provisioners {
    let mut mocked = Provisioners::new();

    for i in 1..3 {
        let rng = &mut StdRng::seed_from_u64(i as u64);
        let sk = SecretKey::random(rng);

        mocked.add_member(HashablePubKey::new(PublicKey::from(&sk)), 1000000, 0, 0);
    }

    mocked
}

async fn perform_basic_run() {
    {
        // Initialize message sources that feeds Consensus.
        let (tx, rx) = mpsc::channel::<MsgNewBlock>(100);
        let (red1_tx, first_red_rx) = mpsc::channel::<MsgReduction>(100);
        let (red2_tx, sec_red_tx) = mpsc::channel::<MsgReduction>(100);

        let producer = spawn_message_producer(tx, red1_tx, red2_tx);

        let mocked = gen_mocked_provisioners();

        let mut c = Consensus::new(rx, first_red_rx, sec_red_tx);
        let n = 5;
        // Run consensus for N rounds
        for r in 0..n {
            c.reset_state_machine();
            c.spin(RoundUpdate::new(r), mocked.clone()).await;
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
