# Block Methods

## Table of Contents

- [Block Methods](#block-methods)
  - [Table of Contents](#table-of-contents)
  - [Block Types](#block-types)
  - [Types](#types)
    - [Type Conventions](#type-conventions)
    - [Request Types](#request-types)
    - [Response Types](#response-types)
  - [Methods](#methods)
    - [Get Blocks Queries](#get-blocks-queries)
    - [getBlockByHash](#getblockbyhash)
      - [Parameters](#parameters-0)
      - [Returns](#returns-0)
      - [Errors](#errors-0)
      - [Examples](#examples-0)
    - [getBlockByHeight](#getblockbyheight)
      - [Parameters](#parameters-1)
      - [Returns](#returns-1)
      - [Errors](#errors-1)
      - [Examples](#examples-1)
    - [getLatestBlock](#getlatestblock)
      - [Parameters](#parameters-2)
      - [Returns](#returns-2)
      - [Errors](#errors-2)
      - [Examples](#examples-2)
    - [getBlockRange](#getblockrange)
      - [Parameters](#parameters-3)
      - [Returns](#returns-3)
      - [Errors](#errors-3)
      - [Examples](#examples-3)
    - [getLatestBlocks](#getlatestblocks)
      - [Parameters](#parameters-4)
      - [Returns](#returns-4)
      - [Errors](#errors-4)
      - [Examples](#examples-4)
    - [getBlocksCount](#getblockscount)
      - [Parameters](#parameters-5)
      - [Returns](#returns-5)
      - [Errors](#errors-5)
      - [Examples](#examples-5)
    - [getBlockPair](#getblockpair)
      - [Parameters](#parameters-6)
      - [Returns](#returns-6)
      - [Errors](#errors-6)
      - [Examples](#examples-6)
    - [Get Block Status](#get-block-status)
    - [getBlockStatus](#getblockstatus)
      - [Parameters](#parameters-7)
      - [Returns](#returns-7)
      - [Errors](#errors-7)
      - [Examples](#examples-7)
    - [Get Block Events](#get-block-events)
    - [getBlockEventsByHash](#getblockeventsbyhash)
      - [Parameters](#parameters-8)
      - [Returns](#returns-8)
      - [Errors](#errors-8)
      - [Examples](#examples-8)
    - [getBlockEventsByHeight](#getblockeventsbyheight)
      - [Parameters](#parameters-9)
      - [Returns](#returns-9)
      - [Errors](#errors-9)
      - [Examples](#examples-9)
    - [getLatestBlockEvents](#getlatestblockevents)
      - [Parameters](#parameters-10)
      - [Returns](#returns-10)
      - [Errors](#errors-10)
      - [Examples](#examples-10)
    - [Get Block Transactions](#get-block-transactions)
    - [getBlockTransactionsByHash](#getblocktransactionsbyhash)
      - [Parameters](#parameters-11)
      - [Returns](#returns-11)
      - [Errors](#errors-11)
      - [Examples](#examples-11)
    - [getBlockTransactionRangeByHash](#getblocktransactionrangebyhash)
      - [Parameters](#parameters-12)
      - [Returns](#returns-12)
      - [Errors](#errors-12)
      - [Examples](#examples-12)
    - [getBlockTransactionsByHeight](#getblocktransactionsbyheight)
      - [Parameters](#parameters-14)
      - [Returns](#returns-14)
      - [Errors](#errors-14)
      - [Examples](#examples-14)
    - [getBlockTransactionRangeByHeight](#getblocktransactionrangebyheight)
      - [Parameters](#parameters-15)
      - [Returns](#returns-15)
      - [Errors](#errors-15)
      - [Examples](#examples-15)
    - [getLastBlockTransactionsByHeight](#getlastblocktransactionsbyheight)
      - [Parameters](#parameters-16)
      - [Returns](#returns-16)
      - [Errors](#errors-16)
      - [Examples](#examples-16)
    - [Specialized Methods](#specialized-methods)
    - [getNextBlockWithPhoenixTransaction](#getnextblockwithphoenixtransaction)
      - [Parameters](#parameters-17)
      - [Returns](#returns-17)
      - [Errors](#errors-17)
      - [Examples](#examples-17)
    - [Get Gas Price](#get-gas-price)
    - [getGasPrice](#getgasprice)
      - [Parameters](#parameters-18)
      - [Returns](#returns-18)
      - [Errors](#errors-18)
      - [Examples](#examples-18)

## Block Types

Blocks in the Dusk Network have two states:

- **Provisional**: Block has been accepted but not yet finalized
- **Final**: Block has reached finality and cannot be reverted

## Types

> NOTE: Some field names and values in this API documentation differ from their internal Rusk codebase counterparts to provide better clarity and consistency with common blockchain API conventions:
>
> 1. Field name changes:
>    Block and Header:
>    - `prev_block_hash` → `previous_hash`: More descriptive
>    - `generator_bls_pubkey` → `validator`: More intuitive for block validator's key
>    - `txroot` → `transactions_root`: More explicit
>    - `iteration` → `sequence`: More descriptive for block sequence in consensus
>
>    Transaction:
>    - `inner` → `transaction_data`: Better describes the transaction payload structure
>
>    Block statistics:
>    - `reward` → `block_reward`: More explicit
>    - `fees` → `total_fees`: More explicit about the sum
>    - `gas_spent` → `total_gas_spent`: More explicit about the total
>    - `max`/`min` → `maximum`/`minimum`: More formal
>
> 2. Enum value changes:
>    - Block status type: Using `"Final"` instead of internal `"Finalized"` for consistency with status values

### Type Conventions

 String representation is used for:

- Hashes (hex-encoded)
- Public keys (base58-encoded)
- Values that can exceed JavaScript's Number.MAX_SAFE_INTEGER (2^53 - 1):
  - Block rewards and fees (atomic units)
  - Gas prices and amounts
  - Chain-dependent values that may grow large

 Number representation is used for:

- User-defined limits and counts
- Protocol versions
- Sequence numbers
- Index values
- Other values that won't exceed 2^53 - 1

### Request Types

```typescript
type BlockByHashRequest = {
    block_hash: string    // 32-byte block hash as hex string
    include_txs: boolean // Include transaction details
}

type BlockByHeightRequest = {
    height: number       // Block height, -1 for latest
    include_txs: boolean // Include transaction details
}

type BlockRangeRequest = {
    start_height: number  // Starting block height
    end_height: number   // Ending block height (inclusive)
    include_txs: boolean // Include transaction details
}

type LatestBlocksRequest = {
    count: number       // Number of blocks to return
    include_txs: boolean // Include transaction details
}

type BlocksCountRequest = {
    finalized_only?: boolean  // Optional, if true returns only finalized blocks count
}

type BlockPairRequest = {
    include_txs?: boolean     // Optional, if true includes transaction details
}

type BlockTransactionsByHashRequest = {
    block_hash: string    // 32-byte block hash as hex string
}

type BlockTransactionsByHeightRequest = {
    height: number           // Block height
}

type BlockTransactionRangeByHashRequest = {
    block_hash: string    // 32-byte block hash as hex string
    start_index: number   // Starting transaction index
    count: number         // The maximum number of transactions to retrieve
}

type BlockTransactionRangeByHeightRequest = {
    height: number           // Block height
    start_index: number     // Starting transaction index
    count: number      // The maximum number of transactions to retrieve
}

type LastBlockTransactionsByHashRequest = {
    block_hash: string    // 32-byte block hash as hex string
    count: number        // Number of transactions to return
    contract_id?: string  // Optional 32-byte contract ID as hex string
}

type LastBlockTransactionsByHeightRequest = {
    height: number          // Block height
    count: number          // Number of transactions to return
}

type BlockStatusRequest = {
    block_height: number     // Block height
}

type NextPhoenixBlockRequest = {
    from_height: number    // Block height to start search from
}

type GasPriceRequest = {
    max_transactions?: number  // Maximum number of transactions to analyze
}
```

### Response Types

```typescript
type BlockHeader = {
    version: number          // Protocol version number
    height: string          // u64, Block height as numeric string
    previous_hash: string   // 32-byte block hash as hex string (64 chars)
    timestamp: number       // Unix timestamp in milliseconds
    hash: string           // 32-byte block hash as hex string (64 chars)
    state_hash: string     // 32-byte state hash as hex string (64 chars)
    validator: string      // BLS public key as base58 string
    transactions_root: string // 32-byte merkle root as hex string (64 chars)
    gas_limit: string      // u64, Maximum gas allowed as numeric string
    seed: string          // 32-byte random seed as hex string (64 chars)
    sequence: number      // Block sequence number in consensus round
}

type BlockStatus = "Final" | "Provisional"  // Block status type

type Block = {
    header: BlockHeader
    status: BlockStatus
    transactions?: TransactionResponse[] // Present if include_txs was true
    transactions_count: number    // Count of transactions in block
    block_reward: string         // u64, Block reward in atomic units as numeric string
    total_fees: string          // u64, Sum of transaction fees in atomic units as numeric string
    total_gas_spent: string     // u64, Total gas consumed as numeric string
}

type BlockEvent = {
    target: string         // 32-byte target address as hex string (64 chars)
    topic: string         // Event topic name
    data: string         // Event data as hex string
    origin: string       // 32-byte transaction hash as hex string (64 chars)
}

type BlocksCountResponse = {
    total: string,           // Total number of blocks as numeric string
}

type BlockPairResponse = {
    latest: Block,           // Latest block
    finalized: Block         // Latest finalized block
}

type TransactionResponse = {
    tx_hash: string           // 32-byte transaction hash as hex string (64 chars)
    version: number       // Protocol version number
    tx_type: "Phoenix" | "Moonlight" // Transaction type
    gas_price: string       // u64, Gas price in atomic units as numeric string
    gas_limit: string       // u64, Gas limit as numeric string
    raw_data: string        // Raw transaction data as hex string
    status: string          // Optional detailed execution status (Pending, Executed, Failed). Populated when status information is available and requested
    block_height: string // Optional u64, Block height as numeric string
    block_hash: string // Optional 32-byte block hash as hex string (64 chars)
    gas_spent: string    // Optional u64, Gas consumed as numeric string
    timestamp: string    // Optional u64, Uniximestamp (in seconds) of the block where the transaction was included as numeric string
    error: string        // Optional error message if status is Failed
    transaction_data: PhoenixTransactionData | MoonlightTransactionData | DeployTransactionData
}

type PhoenixTransactionData = {
    nullifiers: string[]  // Array of 32-byte nullifiers as hex strings (64 chars each)
    outputs: string[]    // Array of 32-byte outputs as hex strings (64 chars each)
    proof: string       // Zero-knowledge proof data validating the transaction as hex strings (64 chars each) 
}

type MoonlightTransactionData = {
    sender: string      // BLS public key as base58 string
    receiver: string    // Optional BLS public key as base58 string
    value: string      // u64, Amount in atomic units as numeric string
    nonce: string      // u64, Transaction nonce as numeric string
    memo: string       // Optional arbitrary data included in the transaction as hex string
}

type DeployTransactionData = {
    bytecode: string      // Hex-encoded WASM bytecode of the deployed contract
    init_args: string    // Optional hex-encoded initialization arguments for the contract's `init` function
}

type GasPriceStats = {
    average: string    // u64, Average gas price in atomic units as numeric string
    maximum: string    // u64, Highest gas price in atomic units as numeric string
    median: string    // u64, Median gas price in atomic units as numeric string
    minimum: string   // u64, Lowest gas price in atomic units as numeric string
}
```

## Methods

### Get Blocks Queries

### getBlockByHash

Returns detailed information about a block identified by its hash.

#### Parameters

[BlockByHashRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| block_hash | string | yes | The block hash as a hex-encoded 32-byte string |
| include_txs | boolean | no | If true, includes transaction details. Defaults to false |

#### Returns

Returns a [Block](#response-types) object.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid hash format (not 64 hex chars) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block with specified hash doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockByHash",
    "params": {
        "block_hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
        "include_txs": true
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "header": {
            "version": 1,                  // Number: protocol version
            "height": "1000",             // String: u64 block height
            "previous_hash": "7d31e39a6318d92fd2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c",
            "timestamp": 1650000000,      // Number: Unix timestamp
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
            "state_hash": "9a51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1234",
            "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",  // Base58 BLS key
            "transactions_root": "aa21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c12345678",
            "gas_limit": "1000000",       // String: u64 gas limit
            "seed": "ab11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c12345678",
            "sequence": 1                  // Number: consensus sequence
        },
        "status": "Final",                // String: "Final" or "Provisional",
        "transactions_count": 2,          // Number: simple count
        "block_reward": "1000000000",    // String: u64 reward
        "total_fees": "50000",           // String: u64 fees
        "total_gas_spent": "46000"       // String: u64 gas
    }
}
```

### getBlockByHeight

Returns detailed information about a block at the specified height.

#### Parameters

[BlockByHeightRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| height | number | yes | Block height. Non-negative values query the specific height, any negative value returns the most recently accepted block |
| include_txs | boolean | no | If true, includes transaction details. Defaults to false |

#### Returns

Returns a [Block](#response-types) object.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid height (negative or too large) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block at specified height doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockByHeight",
    "params": {
        "height": 1000,
        "include_txs": true
    }
}

// Request for latest block
{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "getBlockByHeight",
    "params": {
        "height": -1,
        "include_txs": true
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "header": {
            "version": 1,              // u8, Protocol version number (0-255)
            "height": "1000",          // String: u64 block height
            "previous_hash": "7d31e39a6318d92fd2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c",
            "timestamp": 1650000000,   // Number: Unix timestamp in milliseconds
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
            "state_hash": "9a51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1234",
            "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",
            "transactions_root": "aa21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c12345678",
            "gas_limit": "1000000",    // String: u64 gas limit
            "seed": "ab11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c12345678",
            "sequence": 1              // Number: consensus sequence number
        },
        "status": "Final",                // String: "Final" or "Provisional",
        "transactions_count": 2,      // Number: count of transactions
        "block_reward": "1000000000", // String: u64 block reward
        "total_fees": "50000",       // String: u64 total fees
        "total_gas_spent": "46000"   // String: u64 total gas spent
    }
}
```

### getLatestBlock

Returns information about the most recent block.

#### Parameters

[LatestBlocksRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| include_txs | boolean | no | If true, includes transaction details. Defaults to false |

#### Returns

Returns a [Block](#response-types) object.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getLatestBlock",
    "params": {
        "include_txs": true
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "header": {
            "version": 1,              // u8, Protocol version number (0-255)
            "height": "1050",          // String: u64 block height
            "previous_hash": "7d31e39a6318d92fd2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c",
            "timestamp": 1650001000,   // Number: Unix timestamp in milliseconds
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
            "state_hash": "9a51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1234",
            "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",
            "transactions_root": "aa21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c12345678",
            "gas_limit": "1000000",    // String: u64 gas limit
            "seed": "ab11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c12345678",
            "sequence": 1              // Number: consensus sequence number
        },
        "status": "Final",                // String: "Final" or "Provisional",
        "transactions_count": 1,      // Number: count of transactions
        "block_reward": "1000000000", // String: u64 block reward
        "total_fees": "25000",       // String: u64 total fees
        "total_gas_spent": "21000"   // String: u64 total gas spent
    }
}
```

### getBlocksRange

Returns a sequence of blocks within the specified height range.

#### Parameters

[BlockRangeRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| start_height | number | yes | Starting block height |
| end_height | number | yes | Ending block height (inclusive) |
| include_txs | boolean | no | If true, includes transaction details. Defaults to false |

#### Returns

Returns an array of [Block](#response-types) objects.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid height range (negative, too large, or end < start) |
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockRange",
    "params": {
        "start_height": 1000,
        "end_height": 1002,
        "include_txs": true
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "header": {
                "version": 1,              // u8, Protocol version number (0-255)
                "height": "1000",          // String: u64 block height
                "previous_hash": "7d31e39a6318d92fd2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c",
                "timestamp": 1650000000,   // Number: Unix timestamp in milliseconds
                "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
                "state_hash": "9a51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1234",
                "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",
                "transactions_root": "aa21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c12345678",
                "gas_limit": "1000000",    // String: u64 gas limit
                "seed": "ab11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c12345678",
                "sequence": 1              // Number: consensus sequence number
            },
            "status": "Final",                // String: "Final" or "Provisional",
            "transactions_count": 2,      // Number: count of transactions
            "block_reward": "1000000000", // String: u64 block reward
            "total_fees": "50000",       // String: u64 total fees
            "total_gas_spent": "46000"   // String: u64 total gas spent
        }
        // ... Additional blocks 1001 and 1002 would follow with the same structure
    ]
}
```

### getLatestBlocks

Returns the specified number of most recent blocks.

#### Parameters

[LatestBlocksRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| count | number | yes | Number of latest blocks to return |
| include_txs | boolean | no | If true, includes transaction details. Defaults to false |

#### Returns

Returns an array of [Block](#response-types) objects, ordered from newest to oldest.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid count (zero or too large) |
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getLatestBlocks",
    "params": {
        "count": 2,
        "include_txs": true
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "header": {
                "version": 1,              // u8, Protocol version number (0-255)
                "height": "1002",          // String: u64 block height
                "previous_hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
                "timestamp": 1650000200,   // Number: Unix timestamp in milliseconds
                "hash": "9f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624b",
                "state_hash": "aa51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1235",
                "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",
                "transactions_root": "ba21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c12345679",
                "gas_limit": "1000000",    // String: u64 gas limit
                "seed": "bb11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c12345679",
                "sequence": 1              // Number: consensus sequence number
            },
            "status": "Final",                // String: "Final" or "Provisional",
            "transactions_count": 1,      // Number: count of transactions
            "block_reward": "1000000000", // String: u64 block reward
            "total_fees": "25000",       // String: u64 total fees
            "total_gas_spent": "21000"   // String: u64 total gas spent
        },
        {
            "header": {
                "version": 1,              // u8, Protocol version number (0-255)
                "height": "1001",          // String: u64 block height
                "previous_hash": "7d31e39a6318d92fd2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c",
                "timestamp": 1650000100,   // Number: Unix timestamp in milliseconds
                "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
                "state_hash": "9a51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1234",
                "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",
                "transactions_root": "aa21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c12345678",
                "gas_limit": "1000000",    // String: u64 gas limit
                "seed": "ab11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c12345678",
                "sequence": 1              // Number: consensus sequence number
            },
            "status": "Final",                // String: "Final" or "Provisional",
            "transactions_count": 2,      // Number: count of transactions
            "block_reward": "1000000000", // String: u64 block reward
            "total_fees": "50000",       // String: u64 total fees
            "total_gas_spent": "46000"   // String: u64 total gas spent
        }
    ]
}
```

### getBlocksCount

Returns the total number of blocks in the blockchain.

#### Parameters

[BlocksCountRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| finalized_only | boolean | no | If true, returns only finalized blocks count. Defaults to false |

#### Returns

Returns a [BlocksCountResponse](#response-types) object.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlocksCount",
    "params": {
        "finalized_only": false
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "total": "1050",         // String: Total number of blocks
    }
}
```

### getBlockPair

Returns both the latest candidate block with transaction data and the latest finalized block.

#### Parameters

[BlockPairRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| include_txs | boolean | no | If true, includes transaction details in the finalized block. Defaults to false |

#### Returns

Returns a [BlockPairResponse](#response-types) object.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockPair",
    "params": {
        "include_txs": false
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "latest": {
            "header": {
                "version": 1,
                "height": "1050",
                "previous_hash": "9f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624b",
                "timestamp": 1650002000,
                "hash": "af42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624c",
                "state_hash": "ba51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1236",
                "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",
                "transactions_root": "ca21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c1234567a",
                "gas_limit": "1000000",
                "seed": "cb11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c1234567a",
                "sequence": 1
            },
            "status": "Provisional",                // String: "Final" or "Provisional",
            "transactions_count": 1,
            "block_reward": "1000000000",
            "total_fees": "25000",
            "total_gas_spent": "21000"
        },
        "finalized": {
            "header": {
                "version": 1,
                "height": "1000",
                "previous_hash": "7d31e39a6318d92fd2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c",
                "timestamp": 1650000000,
                "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
                "state_hash": "9a51e39c85f7d2f3c1032bd8118624edb15c5c3f78e7865f102842d0843c1234",
                "validator": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",
                "transactions_root": "aa21e39d96f8e4c2bd8118624edb15c5c3f78e7865f102842d0843c12345678",
                "gas_limit": "1000000",
                "seed": "ab11e39e07f9d5c3bd8118624edb15c5c3f78e7865f102842d0843c12345678",
                "sequence": 1
            },
            "status": "Final",                // String: "Final" or "Provisional",
            "transactions_count": 2,
            "block_reward": "1000000000",
            "total_fees": "50000",
            "total_gas_spent": "46000"
        }
    }
}
```

### Get Block Status

### getBlockStatus

Returns the finalization status of a block identified by its height.

#### Parameters

[BlockStatusRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| block_height | number | yes | The block height |

#### Returns

Returns an object with the [BlockStatus](#response-types) of the block.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid height (non-positive or too large) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block with specified height doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockStatus",
    "params": {
        "block_height": 12345
    }
}

// Response for finalized block
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "status": "Final",          // String: "Final" or "Provisional"
    }
}
```

### Get Block Events

### getBlockEventsByHash

Returns events emitted during block execution for a block identified by its hash.

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| block_hash | string | yes | The block hash as a hex-encoded 32-byte string |

#### Returns

Returns an array of [BlockEvent](#response-types) objects. 

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid hash format (not 64 hex chars) |
| -32603 | Internal error | Database or internal error |
| -32000 | Not found | Block with specified hash doesn't exist or has no associated events |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockEventsByHash",
    "params": {
        "block_hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a"
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "target": "0200000000000000000000000000000000000000000000000000000000000000",     // 32-byte contract address as hex string
            "topic": "moonlight",   // Event topic name
            "data": "7b2273656e646572223a22...",  // Event data as hex string
            "origin": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a"      // 32-byte transaction hash as hex string
        },
        {
            "target": "0200000000000000000000000000000000000000000000000000000000000000",     // 32-byte contract address as hex string
            "topic": "deposit",     // Event topic name
            "data": "7b227265636569766572223a22...",  // Event data as hex string
            "origin": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a"      // 32-byte transaction hash as hex string
        }
    ]
}
```

### getBlockEventsByHeight

Returns events emitted during block execution for a block at the specified height.

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| height | number | yes | Block height to query. |

#### Returns

Returns an array of [BlockEvent](#response-types) objects.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid height (non-positive or too large) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block at specified height doesn't exist or has no associated events |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockEventsByHeight",
    "params": {
        "height": 1000
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "target": "0200000000000000000000000000000000000000000000000000000000000000",     // 32-byte contract address as hex string
            "topic": "moonlight",   // Event topic name
            "data": "7b2273656e646572223a22...",  // Event data as hex string
            "origin": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a"      // 32-byte transaction hash as hex string
        },
        {
            "target": "0200000000000000000000000000000000000000000000000000000000000000",     // 32-byte contract address as hex string
            "topic": "deposit",     // Event topic name
            "data": "7b227265636569766572223a22...",  // Event data as hex string
            "origin": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a"      // 32-byte transaction hash as hex string
        }
    ]
}
```

### getLatestBlockEvents

Returns events emitted during the execution of the latest block.

#### Parameters

None

#### Returns

Returns an array of [BlockEvent](#response-types) objects.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getLatestBlockEvents"
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "target": "0200000000000000000000000000000000000000000000000000000000000000",     // 32-byte contract address as hex string
            "topic": "moonlight",   // Event topic name
            "data": "7b2273656e646572223a22...",  // Event data as hex string
            "origin": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a"      // 32-byte transaction hash as hex string
        },
        {
            "target": "0200000000000000000000000000000000000000000000000000000000000000",     // 32-byte contract address as hex string
            "topic": "deposit",     // Event topic name
            "data": "7b227265636569766572223a22...",  // Event data as hex string
            "origin": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a"      // 32-byte transaction hash as hex string
        }
    ]
}
```

### Get Block Transactions

### getBlockTransactionsByHash

Returns all transactions from a block identified by its hash.

#### Parameters

[BlockTransactionsByHashRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| block_hash | string | yes | The block hash as a hex-encoded 32-byte string |

#### Returns

Returns an array of [TransactionResponse](#response-types) objects.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid hash format (not 64 hex chars) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block with specified hash doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockTransactionsByHash",
    "params": {
        "block_hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "hash": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Moonlight",         // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "21000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "sender": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",      // Base58-encoded BLS public key
                "receiver": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZe",    // Base58-encoded BLS public key
                "value": "1000000000",   // String: u64 amount in atomic units
                "nonce": "42"            // String: u64 nonce
            }
        },
        {
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624b",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Phoenix",           // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "25000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "nullifiers": [
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624c",    // 32-byte nullifier as hex string
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624d"     // 32-byte nullifier as hex string
                ],
                "outputs": [
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624e",    // 32-byte output as hex string
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624f"     // 32-byte output as hex string
                ]
            }
        }
    ]
}
```

### getBlockTransactionRangeByHash

Returns a range of transactions from a block identified by its hash.

#### Parameters

[BlockTransactionRangeByHashRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| block_hash | string | yes | The block hash as a hex-encoded 32-byte string |
| start_index | number | yes | The starting index (0-based) of the transaction range |
| count | number | yes | The maximum number of transactions to retrieve |

#### Returns

Returns an array of [TransactionResponse](#response-types) objects.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid hash format (not 64 hex chars) |
| -32602 | Invalid params | Invalid start_index (too large) |
| -32602 | Invalid params | Invalid count (zero or too large) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block with specified hash doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockTransactionRangeByHash",
    "params": {
        "block_hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",
        "start_index": 0,
        "count": 1,
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "hash": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Moonlight",         // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "21000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "sender": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",      // Base58-encoded BLS public key
                "receiver": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZe",    // Base58-encoded BLS public key
                "value": "1000000000",   // String: u64 amount in atomic units
                "nonce": "42"            // String: u64 nonce
            }
        },
        {
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624b",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Phoenix",           // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "25000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "nullifiers": [
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624c",    // 32-byte nullifier as hex string
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624d"     // 32-byte nullifier as hex string
                ],
                "outputs": [
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624e",    // 32-byte output as hex string
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624f"     // 32-byte output as hex string
                ]
            }
        }
    ]
}
```

### getBlockTransactionsByHeight

Returns all transactions from a block at the specified height.

#### Parameters

[BlockTransactionsByHeightRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| height | number | yes | Block height to query. Positive value representing the specific block height |

#### Returns

Returns an array of [TransactionResponse](#response-types) objects.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block at specified height doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockTransactionsByHeight",
    "params": {
        "height": 1000,
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "hash": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Moonlight",         // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "21000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "sender": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",      // Base58-encoded BLS public key
                "receiver": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZe",    // Base58-encoded BLS public key
                "value": "1000000000",   // String: u64 amount in atomic units
                "nonce": "42"            // String: u64 nonce
            }
        },
        {
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624b",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Phoenix",           // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "25000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "nullifiers": [
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624c",    // 32-byte nullifier as hex string
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624d"     // 32-byte nullifier as hex string
                ],
                "outputs": [
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624e",    // 32-byte output as hex string
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624f"     // 32-byte output as hex string
                ]
            }
        }
    ]
}
```

### getBlockTransactionRangeByHeight

Returns a range of transactions from a block at the specified height.

#### Parameters

[BlockTransactionRangeByHeightRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| height | number | yes | Block height to query. NPositive value representing the specific block height |
| start_index | number | yes | Starting transaction index |
| count | number | yes | The maximum number of transactions to retrieve |

#### Returns

Returns an array of [TransactionResponse](#response-types) objects.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid start_index (too large) |
| -32602 | Invalid params | Invalid count (zero) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block with specified height doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockTransactionRangeByHeight",
    "params": {
        "height": 1000,
        "start_index": 0,
        "end_index": 1,
        "contract_id": "0200000000000000000000000000000000000000000000000000000000000000"  // Optional
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "hash": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Moonlight",         // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "21000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "sender": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",      // Base58-encoded BLS public key
                "receiver": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZe",    // Base58-encoded BLS public key
                "value": "1000000000",   // String: u64 amount in atomic units
                "nonce": "42"            // String: u64 nonce
            }
        },
        {
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624b",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Phoenix",           // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "25000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "nullifiers": [
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624c",    // 32-byte nullifier as hex string
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624d"     // 32-byte nullifier as hex string
                ],
                "outputs": [
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624e",    // 32-byte output as hex string
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624f"     // 32-byte output as hex string
                ]
            }
        }
    ]
}
```

### getLastBlockTransactionsByHeight

Returns the specified number of most recent transactions from a block at the specified height.

#### Parameters

[LastBlockTransactionsByHeightRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| height | number | yes | Block height to query. Positive value representing the specific block height |
| count | number | yes | The maximum number of last transactions to retrieve |

#### Returns

Returns an array of [TransactionResponse](#response-types) objects, ordered from newest to oldest.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | Invalid count (zero) |
| -32603 | Internal error | Database or internal error |
| -32000 | Block not found | Block at specified height doesn't exist |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getLastBlockTransactionsByHeight",
    "params": {
        "height": 1000,
        "count": 2,
        "contract_id": "0200000000000000000000000000000000000000000000000000000000000000"  // Optional
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": [
        {
            "hash": "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624b",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Phoenix",           // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "25000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "nullifiers": [
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624c",    // 32-byte nullifier as hex string
                    "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624d"     // 32-byte nullifier as hex string
                ],
                "outputs": [
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624e",    // 32-byte output as hex string
                    "8f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624f"     // 32-byte output as hex string
                ]
            }
        },
        {
            "hash": "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a",    // 32-byte transaction hash as hex string
            "version": 1,                // u8, Protocol version number (0-255)
            "type": "Moonlight",         // Transaction type: "Phoenix" or "Moonlight"
            "gas_spent": "21000",        // String: u64 gas spent
            "block_height": "1000",      // String: u64 block height
            "transaction_data": {
                "sender": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZd",      // Base58-encoded BLS public key
                "receiver": "DU5KRFf74RyPiZdU8qWngsSk3YRVzPh34B7maWrlrqghkwZe",    // Base58-encoded BLS public key
                "value": "1000000000",   // String: u64 amount in atomic units
                "nonce": "42"            // String: u64 nonce
            }
        }
    ]
}
```

### Specialized Methods

### getNextBlockWithPhoenixTransaction

Returns the height of the next block **after** the given height  that contains at least one Phoenix transaction.

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| from_height | number | yes | starting block height to search from |

#### Returns

Returns the block height as a string, or null if no Phoenix transaction is found.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getNextBlockWithPhoenixTransaction",
    "params": {
        "from_height": 1000
    }
}

// Response when found
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": "1002"     // String: u64 block height
}

// Response when not found
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": null
}
```

### Get Gas Price

### getGasPrice

Returns gas price statistics from mempool transactions to help users set appropriate gas prices for new transactions.

#### Parameters

[GasPriceRequest](#request-types):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| max_transactions | number | no | Maximum number of transactions to analyze. If not specified, all available transactions will be analyzed |

#### Returns

Returns a [GasPriceStats](#response-types) object.

#### Errors

| Code | Message | Description |
|------|---------|-------------|
| -32602 | Invalid params | max_transactions is not a positive integer |
| -32603 | Internal error | Database or internal error |

#### Examples

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getGasPrice",
    "params": {
        "max_transactions": 1000
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "average": "1000000000",    // String: u64, ceiling rounded average gas price
        "maximum": "2000000000",    // String: u64, highest gas price
        "median": "1100000000",     // String: u64, median gas price
        "minimum": "800000000"      // String: u64, lowest gas price
    }
}

// Response when mempool is empty
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "average": "1",    // Default to 1 when no transactions available
        "maximum": "1",
        "median": "1",
        "minimum": "1"
    }
}
```

- **Note**: When the mempool is empty, all gas price values default to "1" to ensure a minimum viable gas price is always available.
