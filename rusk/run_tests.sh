cargo t unspendable --release --features=testwallet -- --nocapture
cargo t wallet --release --features=testwallet -- --nocapture
cargo t multi_transfer --release --features=testwallet -- --nocapture
cargo t erroring_tx_charged_full --release --features=testwallet -- --nocapture
cargo t finalization --release --features=testwallet -- --nocapture
