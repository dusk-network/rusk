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

# host phoenix tests
cargo t host::phoenix::transfer_1_2 --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::transfer_2_2 --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::transfer_3_2 --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::transfer_4_2 --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::transfer_gas_fails --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::alice_ping --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::contract_deposit --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::contract_withdraw --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::convert_to_phoenix_fails --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::convert_to_moonlight --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::convert_wrong_contract_targeted --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::contract_to_contract --release --features=testwallet -- --nocapture --exact
cargo t host::phoenix::contract_to_account --release --features=testwallet -- --nocapture --exact

#host moonlight tests
cargo t host::moonlight::transfer --release --features=testwallet -- --nocapture --exact
cargo t host::moonlight::transfer_with_refund --release --features=testwallet -- --nocapture --exact
cargo t host::moonlight::transfer_gas_fails --release --features=testwallet -- --nocapture --exact
cargo t host::moonlight::alice_ping --release --features=testwallet -- --nocapture --exact
#cargo t host::moonlight::convert_to_phoenix --release --features=testwallet -- --nocapture --exact
