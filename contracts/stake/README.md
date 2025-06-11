<div align="center">

# `📜 Stake Contract`

> The stake contract is a genesis protocol contract that tracks public key stakes. It allows users to stake Dusk tokens subject to a maturation period before becoming eligible for consensus participation.
</div>

## Functions

Below is a list of functions that the stake contract made available. This contract handles staking operations, rewards, and validator management for the consensus mechanism. Each function is annotated with its access restrictions where applicable.

Functions that can only be called from outside the VM are functions, that can only be called by a valid node during transaction processing and block proposal and production. 

These functions are node, i.e., protocol level functions and can never be called directly from user transactions or contracts. Users do not normally call these functions directly. Instead, they interact with the contract through the protocol node & subsequent transfer contract calls.

### `stake`

> Can only be called from the transfer contract

**Description**: Stakes a specified amount of Dusk tokens. The stake has a maturity-period (2 Epochs), after which it is considered valid and the consensus-key becomes eligible to participate in the consensus.

```rust
pub fn stake(&mut self, stake: Stake)
```

### `unstake`

> Can only be called from the transfer contract

**Description**: Unstakes a specified amount from a stake.

```rust
pub fn unstake(&mut self, unstake: Withdraw)
```

### `withdraw`

> Can only be called from the transfer contract

**Description**: Withdraws rewards accumulated by a stake.

```rust
pub fn withdraw(&mut self, withdraw: Withdraw)
```

### `stake_from_contract`

> Can only be called from the transfer contract

> Cannot be called by a root ICC

**Description**: Allows a contract to stake tokens. Similar to the stake function but with different validation requirements.

```rust
pub fn stake_from_contract(&mut self, recv: ReceiveFromContract)
```

### `unstake_from_contract`

**Description**: Allows a contract to unstake tokens.

```rust
pub fn unstake_from_contract(&mut self, unstake: WithdrawToContract)
```

### `withdraw_from_contract`

**Description**: Allows a contract to withdraw accumulated rewards.

```rust
pub fn withdraw_from_contract(&mut self, withdraw: WithdrawToContract)
```

### `get_stake`

**Description**: Retrieves a reference to a stake associated with the given account BLS public key. Returns None if the stake doesn't exist.

```rust
pub fn get_stake(&self, key: &BlsPublicKey) -> Option<&StakeData>
```

### `get_stake_keys`

**Description**: Retrieves the stake keys (account and owner) associated with the given account key. Returns None if the stake doesn't exist.

```rust
pub fn get_stake_keys(&self, key: &BlsPublicKey) -> Option<&StakeKeys>
```

### `burnt_amount`

**Description**: Returns the total amount of tokens that have been burned since genesis through slashing operations.

```rust
pub fn burnt_amount(&self) -> u64
```

### `get_version`

**Description**: Returns the current version of the stake contract, which is defined over a constant (STAKE_CONTRACT_VERSION).

```rust
pub fn get_version(&self) -> u64
```
### `get_config`

**Description**: Returns the stake config (Minimum amount of Dusk that must be staked & number of warnings before being slashed).

```rust
pub fn config(&self) -> &StakeConfig
```

### `stakes`

**Description**: Feeds the host with all existing stakes in the contract.

```rust
pub fn stakes(&self)
```

### `prev_state_changes`

**Description**: Feeds the host with the previous state of changed provisioners.

```rust
pub fn prev_state_changes(&self)
```

### `before_state_transition`

> Can only be called from outside the VM
> Note: The underlying wrapped function has a different name than the exposed state method, hence the name difference.

**Description**:  Clears the previous block state.

```rust
pub fn on_new_block(&mut self) 
```

### `set_config`

> Can only be called from outside the VM
> Note: The underlying wrapped function has a different name than the exposed state method, hence the name difference.

**Description**: Override the stake config with a new one.

```rust
pub fn configure(&mut self, config: StakeConfig)
```

### `insert_stake`

> Can only be called from outside the VM

**Description**: Adds a given stake to the stake contract.

```rust
pub fn insert_stake(&mut self, keys: StakeKeys, stake: StakeData)
```

### `reward`

> Can only be called from outside the VM

**Description**: Rewards multiple accounts with the given rewards.

```rust
pub fn reward(&mut self, rewards: Vec<Reward>)
```

### `slash`

> Can only be called from outside the VM

**Description**: Slashes a specified amount from an account's reward. Increases fault counters and may suspend the stake by shifting its eligibility period.

```rust
pub fn slash(&mut self, account: &BlsPublicKey, to_slash: Option<u64>)
```

### `hard_slash`

> Can only be called from outside the VM

**Description**: Performs a more severe slashing of a stake amount. Unlike regular slashing, this permanently reduces the staked value and burns the tokens. (Currently deactivated)

```rust
pub fn hard_slash(&mut self, account: &BlsPublicKey, to_slash: Option<u64>, severity: Option<u8>)
```

### `set_burnt_amount`

> Can only be called from outside the VM

**Description**: Sets the total amount of tokens that have been burned. *This allows administrative control over the tracked burnt amount value.*

```rust
pub fn set_burnt_amount(&mut self, burnt_amount: u64)
```
