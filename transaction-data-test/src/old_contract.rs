use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionData {
    Call(String),
    Deploy(String),
    Memo(Memo),
}

pub type Memo = Vec<u8>;