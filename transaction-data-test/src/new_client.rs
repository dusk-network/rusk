use bincode;
use serde::{Deserialize, Serialize};

use crate::old_contract::{Memo, TransactionData as OldTransactionData}; // Assuming old_contract is in the same crate

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionData {
    Call(String),
    Deploy(String),
    Memo(Memo),
    Blob(Memo), // New variant added in the new client
}

pub fn run_test() {
    // Simulate a client with a new type
    let new_enum = TransactionData::Blob("hello blob".as_bytes().to_vec());

    let serialized =
        bincode::serialize(&new_enum).expect("serialization failed");

    // Simulate deserialization on a contract with the old enum
    let result: Result<OldTransactionData, _> =
        bincode::deserialize(&serialized);

    // Expect an error
    match result {
        Ok(val) => println!("❌ Unexpected success: {:?}", val),
        Err(e) => println!("✅ Expected failure: {:?}", e), // << invalid value: integer `3`, expected variant index 0 <= i < 3
    }
}
