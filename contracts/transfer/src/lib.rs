#![no_std]
use dataview::Pod;
use phoenix_abi::{Input, Note, Proof, PublicKey};

type Inputs = [Input; Input::MAX];
type Notes = [Note; Note::MAX];

#[repr(C)]
#[derive(Debug)]
pub struct TransferArgs(Inputs, Notes, Proof);
unsafe impl Pod for TransferArgs {}

#[repr(C)]
#[derive(Debug)]
pub struct ApproveArgs(Inputs, Notes, PublicKey, u64, Proof);
unsafe impl Pod for ApproveArgs {}

#[repr(C)]
#[derive(Debug)]
pub struct FromArgs(PublicKey, PublicKey, u64);
unsafe impl Pod for FromArgs {}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
enum OpCode {
    None,
    Transfer,
    Approve,
    TransferFrom,
}

unsafe impl Pod for OpCode {}

impl Default for OpCode {
    fn default() -> Self {
        OpCode::None
    }
}

#[no_mangle]
pub fn call() {
    let code: OpCode = dusk_abi::opcode::<OpCode>();

    dusk_abi::ret::<i32>(match code {
        OpCode::Transfer => transfer(dusk_abi::argument()),
        OpCode::Approve => approve(dusk_abi::argument()),
        OpCode::TransferFrom => transfer_from(dusk_abi::argument()),
        _ => 0,
    });
}

pub fn transfer(TransferArgs(inputs, notes, proof): TransferArgs) -> i32 {
    if !phoenix_abi::verify(&inputs, &notes, &proof) {
        return 0;
    }
    phoenix_abi::store(&inputs, &notes, &proof) as i32
}

pub fn approve(
    ApproveArgs(inputs, notes, pk, value, proof): ApproveArgs,
) -> i32 {
    if !phoenix_abi::verify(&inputs, &notes, &proof) {
        return 0;
    }

    phoenix_abi::store(&inputs, &notes, &proof);
    let current_value = dusk_abi::get_storage(&pk).unwrap_or(0);
    dusk_abi::set_storage(&pk, value + current_value);
    1
}

pub fn transfer_from(FromArgs(sender, recipient, value): FromArgs) -> i32 {
    let approved_value = dusk_abi::get_storage(&sender).unwrap_or(0);
    if value > approved_value {
        return 0;
    }

    dusk_abi::set_storage(&sender, approved_value - value);
    phoenix_abi::credit(value, &recipient);
    1
}
