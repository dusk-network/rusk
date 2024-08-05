# Transfer Contract

## Introduction

The transfer contract is one of Dusk's genesis smart contracts, and is responsible for defining the
rules that a transaction must follow to be executed on the network. It serves as an entrypoint for
every transaction, and can therefore be though as the definition of 'Dusk' as a cryptocurrency.

Like any smart contract on Dusk, the transfer contract exposes a set of functions and maintains
state. In this document we will go through the functions it exposes, and discuss their intended
use, together with their arguments, returns, and calling convention. We will also specify the state
the functions operate on, and describe the types of mutation to the state each of them may perform.

## State

TODO

## Functions

The functions of the transfer contract may be broadly categorized in three different ways, differing
in how they're meant to be used, and by whom: management calls, contract calls, and queries.

Unless otherwise noted, the structures referred to are defined in the [`execution_core::transfer`]
module, and serialized using [`rkyv`]. In the case(s) that the function takes as argument or returns
a tuple of values, `rkyv` tuple serialization is used.

[`execution_core::transfer`]: https://github.com/dusk-network/rusk/blob/master/execution-core/src/transfer.rs
[`rkyv`]: https://github.com/rkyv/rkyv

### Management Calls

Perhaps the simplest to understand are so called management calls, so called because they're only
meant to be called directly by the host. If called by another contract, including the transfer
contract itself, they will produce a panic.

They are only callable by the host because they rely on guarantees that the contract itself is
unable to provide on its own, such as the case of `spend_and_execute` and `refund`, that must be
called in sequence.

#### `spend_and_execute: Transaction -> Result<Vec<u8>, ContractError>`
#### `refund: u64 -> ()`
#### `push_note: (u64, Note) -> Note`
#### `update_root: () -> ()`
#### `add_account_balance: (AccountPublicKey, u64) -> ()`
#### `sub_account_balance: (AccountPublicKey, u64) -> ()`
#### `add_contract_balance: (ContractId, u64) -> ()`

### Contract Calls

Contract calls are meant to be callable by other contracts to perform certains actions with Dusk
during their execution. They may restrict, on a per function basis, which contract is allowed to
call them.

Of particular note are the pair of `deposit` and `withdraw`, allowing for Dusk to be deposited to
and withdrawn from a contract, respectively.

#### `deposit: u64 -> ()`
#### `withdraw: Withdraw -> ()`
#### `convert: Withdraw -> ()`
#### `mint: Withdraw -> ()`
#### `sub_contract_balance: (ContractId, u64) -> Result<(), Error>`

### Queries

Queries may be called by either contracts or the host, and are used to request information about the
[state] managed by the transfer contract. They may be used by wallets to sync with the current state
of the network, or by contracts wishing to do the same.

[state]: #state

<!-- Normal queries -->

#### `root: () -> BlsScalar`
#### `account: AccountPublicKey -> AccountData`
#### `contract_balance: ContractId -> u64`
#### `opening: u64 -> Opening<(), NOTES_TREE_DEPTH>`
#### `existing_nullifiers: Vec<BlsScalar> -> BlsScalar`
#### `num_notes: () -> u64`

<!-- Feeder queries -->

#### `leaves_from_height: u64 -> ()`
#### `leaves_from_pos: u64 -> ()`
