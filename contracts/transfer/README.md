<div align="center">

# `🔄 Transfer Contract`

> The transfer contract is a genesis protocol contract, acting as the entrypoint for any transaction happening on the network
</div>

## Functions

Below is a list of functions provided by the transfer contract. This contract manages token transfers, balances and transaction-related operations within the Dusk blockchain. Each function is annotated with its access restrictions where applicable.

Functions that can only be called from outside the VM are functions, that can only be called by a valid node during transaction processing, block proposal and production. 

These functions are node, i.e., protocol level functions and can never be called directly from user transactions or contracts. Users do not normally call these functions directly. Instead, they interact with the contract through the protocol node, by submitting transactions to the mempool.

### `mint`

> *Can only be called by the stake contract*

**Description**: Mints new Dusk tokens according to the reward withdrawal request from the stake contract. The tokens are minted either to a Phoenix address or a Moonlight account as specified in the withdrawal. This function increases the total amount of circulating Dusk and is only called during the execution of the stake contracts `withdraw` function. Any amount minted conforms to the consensus emission schedule.

```rust
pub fn mint(&mut self, mint: Withdraw) -> ()
```

### `mint_to_contract`

> *Can only be called by the stake contract*

**Description**: Similar to the `mint` function, with the difference that the tokens are minted to a contract. This function increases the total amount of circulating Dusk and is intended to be called during the execution of the `withdraw_to_contract` function. Any amount minted conforms to the consensus emission schedule.

```rust
pub fn mint_to_contract(&mut self, mint: ContractToContract) -> ()
```

### `deposit`

**Description**: Picks up funds for a contract's balance that were previously deposited on the state by either a moonlight or phoenix transaction. If a deposit is placed on the state but the contract fails to pick it up via this function in a subsequent ICC, the deposit on the state is transferred back.

```rust
pub fn deposit(&mut self, value: u64) -> ()
```

### `withdraw`

**Description**: Withdraws funds from a contract's balance to either a transparent Phoenix note or a Moonlight account.

```rust
pub fn withdraw(&mut self, withdraw: Withdraw) -> ()
```

### `convert`
> Can only be called by the transfer contract

**Description**: Performs an atomic conversion transaction between Phoenix notes and Moonlight balances by utilizing deposits sent to the transfer contract itself. During a conversion, the specified value is transformed from moonlight balance to a phoenix-note, and vice versa. In a phoenix-to-moonlight conversion, gas is paid with the nullified phoenix-notes. When converting moonlight balance into phoenix-notes, gas is paid by the moonlight balance.

```rust
pub fn convert(&mut self, convert: Withdraw) -> ()
```

### `contract_to_contract`

**Description**: Transfers funds from one contract's balance to another and calls a function on the receiving contract (specified in the transfer argument). Contracts capable of receiving funds from other contracts are expected to expose the function specified by the sender, which is called using a `ReceiveFromContract` as argument.

```rust
pub fn contract_to_contract(&mut self, transfer: ContractToContract) -> ()
```

### `contract_to_account`

**Description**: Transfers funds from a contract balance to a moonlight account.

```rust
pub fn contract_to_account(&mut self, transfer: ContractToAccount) -> ()
```

### `root`

**Description**: Returns the current root of the merkle tree of all phoenix-notes as a cryptographic commitment to the current state. The root is essential for verifying note inclusion in the tree and validating phoenix transactions.

```rust
pub fn root(&self) -> BlsScalar
```

### `account`

**Description**: Retrieves the moonlight account data i.e., balance and nonce that is associated with the specified public key.

```rust
pub fn account(&self, key: &AccountPublicKey) -> AccountData
```

### `contract_balance`

**Description**: Returns the balance of the specified contract.

```rust
pub fn contract_balance(&self, contract_id: &ContractId) -> u64
```

### `opening`

**Description**: Retrieves the merkle opening for a note-hash at the specified position. Returns None if the position is invalid or the note doesn't exist.

```rust
pub fn opening(&self, pos: u64) -> Option<NoteOpening>
```

### `existing_nullifiers`

**Description**: Based on a given list of nullifiers, returns only the nullifiers that already exist in the contract, i.e. the nullifiers of the notes that have been spent already. Nullifiers "nullify" notes and prevent double-spending attempts of already spent notes.

```rust
pub fn existing_nullifiers(&self, nullifiers: Vec<BlsScalar>) -> Vec<BlsScalar>
```

### `num_notes`

**Description**: Returns the total amount of notes in the tree.

```rust
pub fn num_notes(&self) -> u64
```

### `chain_id`

**Description**: Returns the chain ID of the current blockchain. Will be `1` in the case of Dusk mainnet.

```rust
pub fn chain_id(&self) -> u8
```

### `leaves_from_height`

**Description**: Feeds the host with all leaves in the tree starting from the given block height.

```rust
pub fn leaves_from_height(&self, height: u64)
```

### `leaves_from_pos`

**Description**: Feeds the host with all leaves in the tree starting from the given tree position.

```rust
pub fn leaves_from_pos(&self, pos: u64)
```

### `sync`

**Description**: Feeds the host with leaves starting from a position, with an optional limit.

```rust
pub fn sync(&self, from: u64, count_limit: u64)
```

### `sync_nullifiers`

**Description**: Feeds the host with nullifiers starting from a position (skipping entries before), with an optional limit.

```rust
pub fn sync_nullifiers(&self, from: u64, count_limit: u64)
```

### `sync_contract_balances`

**Description**: Feeds the host with all contract balances, with an optional limit.

```rust
pub fn sync_contract_balances(&self, from: u64, count_limit: u64)
```

### `sync_accounts`

**Description**: Feeds the host with account data (balances & nonces), with an optional limit.

```rust
pub fn sync_accounts(&self, from: u64, count_limit: u64)
```

### `spend_and_execute`
> *Can only be called from outside the VM*

**Description**: This function is the main entry point for all transaction executions and manages the complete transaction lifecycle from spending Dusk to executing contract calls and paying gas. As such, it is handling both Phoenix and Moonlight transactions, although Phoenix & Moonlight are strictly separated. The spending phase will either go into `spend_phoenix` or `spend_moonlight` based on the transaction type.

```rust
pub fn spend_and_execute(&mut self, tx: Transaction) -> Result<Vec<u8>, ContractError>
```

### `refund`

> *Can only be called from outside the VM*

**Description**: Refunds remaining gas and unclaimed deposits after transaction execution. Emits events with transaction details and refund information.

```rust
pub fn refund(&mut self, gas_spent: u64)
```

### `push_note`

> *Can only be called from outside the VM*

**Description**: Adds a new note to the tree with the specified block height. This is related to the phoenix UTXO model, allowing new outputs to be added.

```rust
pub fn push_note(&mut self, block_height: u64, note: Note) -> Option<Note>
```

### `update_root`

> *Can only be called from outside the VM*

**Description**: Updates the list of tree roots with the current tree root. Enables future verification of notes against historical tree states.

```rust
pub fn update_root(&mut self)
```

### `add_account_balance`

> *Can only be called from outside the VM*

**Description**: Adds Dusk to an moonlight account's balance, creating the account if it doesn't exist.

```rust
pub fn add_account_balance(&mut self, key: &AccountPublicKey, value: u64)
```

### `sub_account_balance`

> *Can only be called from outside the VM*

**Description**: Subtracts Dusk from a moonlight account's balance if the account exists.

```rust
pub fn sub_account_balance(&mut self, key: &AccountPublicKey, value: u64)
```

### `add_contract_balance`

> *Can only be called from outside the VM*

**Description**: Adds Dusk to a contract's balance. Used for deposits and transfers between contracts.

```rust
pub fn add_contract_balance(&mut self, contract: ContractId, value: u64)
```

### `sub_contract_balance`

> *Can only be called by the stake contract*

**Description**: Subtracts Dusk from a contract's balance. This is **only** called by the stake contract in the case of slashing.

```rust
pub(crate) fn sub_contract_balance(&mut self, address: &ContractId, value: u64) -> Result<(), Error>
```
