use crate::state::{ziputil, PROVISIONERS};
use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use microkelvin::{Backend, BackendCtor, DiskBackend, Persistence};
use once_cell::sync::Lazy;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::dusk::*;
use rusk_vm::{Contract, NetworkState, NetworkStateId};
use stake_contract::{Stake, StakeContract, MINIMUM_STAKE};
use std::{
    env,
    error::Error,
    fs,
    io::{self, Read},
};
use transfer_contract::TransferContract;

/// Amount of the note inserted in the genesis state.
const GENESIS_DUSK: Dusk = dusk(1_000.0);

/// Faucet note value.
const FAUCET_DUSK: Dusk = dusk(1_000_000_000.0);

pub static DUSK_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../dusk.psk");
    PublicSpendKey::from_bytes(bytes).expect("dusk should have a valid key")
});

pub static FAUCET_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../faucet.psk");
    PublicSpendKey::from_bytes(bytes).expect("faucet should have a valid key")
});

fn diskbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(|| {
        //create a Pathbuf for a temporary directory in OUT_DIR folder
        let out_dir = env::var("OUT_DIR").unwrap();
        let mut dir = std::path::PathBuf::from(out_dir);
        dir.push("build-state");
        dir.push("state");
        let _ = fs::create_dir_all(dir.clone());

        // let dir = rusk_profile::get_rusk_state_dir()
        //     .expect("Failed to get Rusk profile directory");

        fs::remove_dir_all(&dir)
            .or_else(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .expect("Failed to clean up Network State directory");

        fs::create_dir_all(&dir)
            .expect("Failed to create Network State directory");

        DiskBackend::new(dir)
    })
}

/// Creates a new transfer contract state with a single note in it - ownership
/// of Dusk Network. If `testnet` is true an additional note - ownership of the
/// faucet address - is added to the state.
fn genesis_transfer(testnet: bool) -> TransferContract {
    let mut transfer = TransferContract::default();
    let mut rng = StdRng::seed_from_u64(0xdead_beef);
    let note = Note::transparent(&mut rng, &DUSK_KEY, GENESIS_DUSK);

    transfer
        .push_note(0, note)
        .expect("Genesis note to be pushed to the state");

    if testnet {
        let note = Note::transparent(&mut rng, &*FAUCET_KEY, FAUCET_DUSK);
        transfer
            .push_note(0, note)
            .expect("Faucet note to be pushed in the state");
    }

    transfer
        .update_root()
        .expect("Root to be updated after pushing genesis note");

    let stake_amount = stake_amount(testnet);
    let stake_balance = stake_amount * PROVISIONERS.len() as u64;

    transfer
        .add_balance(rusk_abi::stake_contract(), stake_balance)
        .expect("Stake contract balance to be set with provisioner stakes");

    transfer
}

const fn stake_amount(testnet: bool) -> Dusk {
    match testnet {
        true => dusk(2_000_000.0),
        false => MINIMUM_STAKE,
    }
}

/// Creates a new stake contract state with preset stakes added for the
/// staking/consensus keys in the `keys/` folder. The stakes will all be the
/// same and the minimum amount.
fn genesis_stake(testnet: bool) -> StakeContract {
    let mut stake_contract = StakeContract::default();

    let stake_amount = stake_amount(testnet);

    for provisioner in PROVISIONERS.iter() {
        let stake = Stake::with_eligibility(stake_amount, 0, 0);
        stake_contract
            .insert_stake(*provisioner, stake)
            .expect("Genesis stake to be pushed to the stake");
    }
    println!(
        "{} Added {} provisioners",
        ("Generating"),
        PROVISIONERS.len()
    );

    stake_contract
}

pub fn deploy<B>(
    testnet: bool,
    ctor: &BackendCtor<B>,
) -> Result<NetworkStateId, Box<dyn Error>>
where
    B: 'static + Backend,
{
    Persistence::with_backend(ctor, |_| Ok(()))?;

    println!("{} new network state", ("Generating"));

    let transfer_path =
        "../target/wasm32-unknown-unknown/release/transfer_contract.wasm";
    let transfer_bytes = fs::read(transfer_path).map_err(|e| {
        println!("couldn't read {}: {}", transfer_path, e);
        e
    })?;
    let transfer =
        Contract::new(genesis_transfer(testnet), &transfer_bytes[..]);

    let stake_path =
        "../target/wasm32-unknown-unknown/release/stake_contract.wasm";
    let stake_bytes = fs::read(stake_path).map_err(|e| {
        println!("couldn't read {}: {}", stake_path, e);
        e
    })?;
    let stake = Contract::new(genesis_stake(testnet), &stake_bytes[..]);

    let mut network = NetworkState::default();

    println!("{} Genesis Transfer Contract state", ("Deploying"));

    network
        .deploy_with_id(rusk_abi::transfer_contract(), transfer)
        .expect("Genesis Transfer Contract should be deployed");

    println!("{} Genesis Stake Contract state", ("Deploying"));

    network
        .deploy_with_id(rusk_abi::stake_contract(), stake)
        .expect("Genesis Transfer Contract should be deployed");

    println!("{} network state", ("Storing"));

    network.commit();
    network.push();

    println!("{} {}", ("Root"), hex::encode(network.root()));

    let state_id = network.persist(ctor).expect("Error in persistence");

    Ok(state_id)
}

pub(super) fn build_state() -> Result<Vec<u8>, Box<dyn Error>> {
    // check if the value of the environment variable named RUSK_BUILD_TESTNET
    // is true

    let testnet = env::var("RUSK_BUILD_TESTNET")
        .map(|v| v.parse::<bool>().unwrap_or(false))
        .unwrap_or(false);

    println!("{} new state", ("Building"));
    let state_id = deploy(testnet, &diskbackend())
        .expect("Failed to deploy network state");

    println!("{} persisted id", ("Storing"));
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut build_path = std::path::PathBuf::from(out_dir.clone());

    build_path.push("build-state");

    let mut id_path = std::path::PathBuf::from(build_path.clone());
    id_path.push("state.id");
    state_id.write(&id_path)?;

    let mut out_path = std::path::PathBuf::from(out_dir);
    out_path.push("state.zip");

    ziputil::zip_dir(
        build_path.to_str().unwrap(),
        out_path.to_str().unwrap(),
        zip::CompressionMethod::Deflated,
    )?;

    //read file out.zip to vec
    let mut file = fs::File::open(out_path.to_str().unwrap())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
