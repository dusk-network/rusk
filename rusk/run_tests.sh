# services tests
cargo t unspendable --release --features=testwallet -- --nocapture
cargo t wallet --release --features=testwallet -- --nocapture # transfer
cargo t multi_transfer --release --features=testwallet -- --nocapture # multi_transfer & sequential_nonce
cargo t erroring_tx_charged_full --release --features=testwallet -- --nocapture # gas_behavior
cargo t finalization --release --features=testwallet -- --nocapture
cargo t contract_deployment --release --features=testwallet -- --nocapture
cargo t init --release --features=testwallet -- --nocapture
cargo t conversion --release --features=testwallet -- --nocapture
cargo t owner_calls --release --features=testwallet -- --nocapture

# host tests
cargo t transfer_1_2 --release --features=testwallet -- --nocapture
cargo t transfer_2_2 --release --features=testwallet -- --nocapture
cargo t transfer_3_2 --release --features=testwallet -- --nocapture
cargo t transfer_4_2 --release --features=testwallet -- --nocapture
cargo t transfer_gas_fails --release --features=testwallet -- --nocapture
#cargo t alice_ping --release --features=testwallet -- --nocapture
