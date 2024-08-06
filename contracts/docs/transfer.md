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

#### `spend_and_execute(Transaction) -> Result<Vec<u8>, ContractError>`

The main entrypoint for Dusk transactions, taking in either a Moonlight or Phoenix `transaction`.
Checks if the transaction is valid, spends **all** the available funds, and executes the contract
call if present in the transaction.

It returns the result of the contract call, as returned by the called contract, if the call exists
and succeeds. If the call exists and fails, an error containing the reason for failure is returned.
The call may fail its execution for a myriad of reasons, the most common being it running out of gas
during execution or a panic by the called contract.
If there is no call present, an empty vector of bytes is returned.

If this function panics or runs out of gas, a transaction should be considered *invalid*, meaning it
has no chance of ever being executed and should be discarded.

The gas spent during the execution of this function forms part of the gas that gets charged to a
transaction, the other part being a possible contract deployment. Since it spends all available
funds, a subsequent call to [`refund`] is **required** to return unspent funds to the user.

[`refund`]: #refundu64

#### `refund(u64)`

This function is responsible for computing the unspent funds of a transaction and returning them to
the appropriate account or address, depending on if the executed transaction was Moonlight or
Phoenix.

It must be called after a successful [`spend_and_execute`] call. It is guaranteed to succeed if the
passed `gas_spent` does not exceed a transaction's gas limit.

[`spend_and_execute`]: #spend_and_executetransaction---resultvecu8-contracterror

#### `push_note(u64, Note) -> Note`

Inserts a `note` at the next position in merkle tree, with the given `block_height` attached. A
sequence of calls to be function must be succeeded by a call to [`update_root`] to update the root
of the merkle tree. The caller is responsible for ensuring that block height remains monotonically
increasing in the tree.

This function is primarily intended for inserting notes into the tree at genesis.

[`update_root`]: #update_root

#### `update_root()`

Updates the root of the merkle tree. Since merkle tree insertion is lazy, this function must be
explicitly called after notes have been inserted in the tree.

This means that it must be called once after every block ingestion, meaning after a sequence of
[`spend_and_execute`] and [`refund`] calls.

#### `add_account_balance(AccountPublicKey, u64)`

Adds `balance` to the account with the given `public_key`. This function is intended for
manipulating balances at genesis.

#### `sub_account_balance(AccountPublicKey, u64)`

Subtracts `balance` to the account with the given `public_key`. This function is intended for
manipulating balances at genesis.

#### `add_contract_balance(ContractId, u64)`

Adds `balance` to the contract with the given `id`. This function is intended for manipulating
contract balances at genesis.

### Contract Calls

Contract calls are meant to be callable by other contracts to perform certains actions with Dusk
during their execution. They may restrict, on a per function basis, which contract is allowed to
call them.

Of particular note are the pair of `deposit` and `withdraw`, allowing for Dusk to be deposited to
and withdrawn from a contract, respectively.

#### `deposit(u64)`

This function can be called by any contract and is used to collect the deposit a transaction may
have left for it. Only the contract to which the deposit is destined may call this function, and the
`value` argument must match the deposit amount.

Depositors - i.e. transactions that perform a deposit - are expected to contain a call directly to
the contract function handling deposits. This function is meant to be called by the contract
*collecting* the deposit, not the depositor themselves.

If it succeeds, the balance of the contract will be increased by the deposited value.

#### `withdraw(Withdraw)`

Withdraws can be called by any contract to allow a transactor to withdraw from their balance. The
`withdraw` argument contains information about the contract to withdraw from, - which must match the
contract calling this function - the value to withdraw, and the mode of withdrawal (Moonlight or
Phoenix).

Withdrawers - i.e. transaction that want to withdraw from a contract - are expect to contain a call
directly to the contract function handling withdrawals. This function is meant to be called by the
contract *allowing* the withdrawal, not the withdrawer themselves.

If it succeeds, the balance of the contract will be decreased by the withdrawal value, and the same
value is credited to the withdrawers mode of choice.

#### `convert(Withdraw)`

This function can be called by transactors to convert between Moonlight and Phoenix Dusk. It can be
conceptualized as a simultaneous [`deposit`] to and a [`withdrawal`] from the transfer contract. In
contrast to these two functions, however, it is meant to, and can only be, called directly by the
transactor.

If it succeeds the balance of the contract remains constant, and the value is converted from Phoenix
to Moonlight or vice-versa.

[`deposit`]: #depositu64
[`withdrawal`]: #withdrawwithdraw

#### `mint(Withdraw)`

This function is designed to be called solely by the stake contract - another genesis contract - to
allow it to be able to withdraw the rewards for participating in the consensus.

If it succeeds the transactor will be credited with the `withdraw`n reward.

#### `sub_contract_balance(ContractId, u64) -> Result<(), Error>`

This function is designed to be called solely by the stake contract - another genesis contract - to
allow it to be able to deduct from its own balance in the event of a slash.

If it succeeds, the `contract` will have the given `value` deducted from their balance.

### Queries

Queries may be called by either contracts or the host, and are used to request information about the
[state] managed by the transfer contract. They may be used by wallets to sync with the current state
of the network, or by contracts wishing to do the same.

[state]: #state

<!-- Normal queries -->

#### `root() -> BlsScalar`
#### `account(AccountPublicKey) -> AccountData`
#### `contract_balance(ContractId) -> u64`
#### `opening(u64) -> Opening<(), NOTES_TREE_DEPTH>`
#### `existing_nullifiers(Vec<BlsScalar>) -> BlsScalar`
#### `num_notes() -> u64`

<!-- Feeder queries -->

#### `leaves_from_height(u64)`
#### `leaves_from_pos(u64)`
