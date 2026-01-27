cargo t unspendable --release --features=testwallet -- --nocapture
cargo t wallet --release --features=testwallet -- --nocapture # transfer
cargo t multi_transfer --release --features=testwallet -- --nocapture # multi_transfer & sequential_nonce
cargo t erroring_tx_charged_full --release --features=testwallet -- --nocapture # gas_behavior
cargo t finalization --release --features=testwallet -- --nocapture
cargo t contract_deployment --release --features=testwallet -- --nocapture
cargo t init --release --features=testwallet -- --nocapture
