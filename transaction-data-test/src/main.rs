mod old_contract;
mod new_client;

fn main() {
    println!("Running enum compatibility test...");
    new_client::run_test();
}