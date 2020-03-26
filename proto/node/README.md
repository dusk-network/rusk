# Protocol Documentation - Node

## Table of contents

- [mempool.proto](#mempool.proto)
	- [TxType](#txtype)
	- [Tx](#tx)
	- [SelectRequest](#selectrequest)
	- [SelectResponse](#selectresponse)

- [wallet.proto](#wallet.proto)
	- [PubKey](#pubkey)
	- [CreateRequest](#createrequest)
	- [LoadRequest](#loadrequest)
	- [LoadResponse](#loadresponse)
	- [ConsensusTxRequest](#consensustxrequest)
	- [TransferRequest](#transferrequest)
	- [TransferResponse](#transferresponse)
	- [WalletStatusResponse](#walletstatusresponse)
	- [SyncProgressResponse](#syncprogressresponse)
	- [BalanceResponse](#balanceresponse)
	- [Direction](#direction)
	- [TxRecord](#txrecord)
	- [TxHistoryResponse](#txhistoryresponse)

- [node.proto](#node.proto)
	- [EmptyRequest](#emptyrequest)
	- [GenericResponse](#genericresponse)
	- [Methods](#methods)

## mempool.proto

### TxType

| Name | Number | Description |
| ---- | ------ | ----------- |
| COINBASE | 0 | A coinbase transaction |
| BID | 1 | A bid transaction |
| STAKE | 2 | A stake transaction |
| STANDARD | 3 | A standard transaction |
| TIMELOCK | 4 | A timelock transaction |
| CONTRACT | 5 | A contract transaction |

### Tx

| Field | Type | Description |
| ----- | ---- | ----------- |
| type | [TxType](#txtype) | Identifier for the type of transaction |
| id | string | Hex-encoded hash of the transaction |
| lock_time | fixed64 | The amount of blocks the transaction will be locked up for upon acceptance |

### SelectRequest

| Field | Type | Description |
| ----- | ---- | ----------- |
| types | [TxType](#txtype) (repeated) | Types of transactions that the caller wishes to receive |
| id | string | Hex-encoded hash of the transaction that the caller wants to see |

### SelectResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| result | [Tx](#Tx) (repeated) | Selected transactions |

## wallet.proto

### PubKey

| Field | Type | Description |
| ----- | ---- | ----------- |
| public_key | bytes | A wallet public key |

### CreateRequest

| Field | Type | Description |
| ----- | ---- | ----------- |
| password | string | The password to encrypt the wallet file with |
| seed | bytes | An optional seed to use for creating the wallet file |

### LoadRequest

| Field | Type | Description |
| ----- | ---- | ----------- |
| password | string | The password to decrypt the wallet file with |

### LoadResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| key | [PubKey](#pubkey) | The loaded wallet public key |

### ConsensusTxRequest

| Field | Type | Description |
| ----- | ---- | ----------- |
| amount | fixed64 | The amount of DUSK to stake/bid, in atomic units (no decimal) |
| lock_time | fixed64 | The amount of blocks to lock the stake/bid up for, once accepted into the blockchain |

### TransferRequest

| Field | Type | Description |
| ----- | ---- | ----------- |
| amount | fixed64 | The amount of DUSK to transfer, in atomic units (no decimal) |
| address | bytes | The address of the recipient of the transfer |

### TransferResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| hash | bytes | The hash of the created transaction |

### WalletStatusResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| loaded | bool | True if wallet is loaded, false if not |

### SyncProgressResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| progress | float | Synchronization progress percentage |

### BalanceResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| unlockedBalance | fixed64 | The amount of unlocked DUSK in the wallet, in atomic units (no decimal) |
| lockedBalance | fixed64 | The amount of locked DUSK in the wallet, in atomic units (no decimal) |

### Direction

| Name | Number | Description |
| ---- | ------ | ----------- |
| OUT | 0 | An outbound transaction |
| IN | 1 | An inbound transaction |

### TxRecord

| Field | Type | Description |
| ----- | ---- | ----------- |
| direction | [Direction](#direction) | Directional nature of the transaction |
| timestamp | int64 | UNIX timestamp of when the transaction was created |
| height | fixed64 | The block height at which the transaction was included into the blockchain |
| type | [TxType](#txtype) | The type of transaction |
| amount | fixed64 | The amount of DUSK transferred, in atomic units (no decimal) |
| unlockHeight | fixed64 | The height at which this transaction will unlock |

### TxHistoryResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| records | [TxRecord](#txrecord) (repeated) | The transaction history of a wallet |

## node.proto

### EmptyRequest

| Field | Type | Description |
| ----- | ---- | ----------- |

Just an empty message, for requests that do not need parameters.

### GenericResponse

| Field | Type | Description |
| ----- | ---- | ----------- |
| response | string | Can hold any generic text-based response for the caller |

### Methods

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ----------- |
| CreateWallet | [CreateRequest](#createrequest) | [LoadResponse](#loadresponse) | Create a wallet file and load it |
| LoadWallet | [LoadRequest](#loadrequest) | [LoadResponse](#loadresponse) | Load a wallet file |
| CreateFromSeed | [CreateRequest](#createrequest) | [LoadResponse](#loadresponse) | Create a wallet file from the given seed, and load it |
| ClearWalletDatabase | [EmptyRequest](#emptyrequest) | [GenericResponse](#genericresponse) | Clear the wallet database, removing knowledge of all inputs belonging to the loaded wallet |
| Transfer | [Transfer](#transfer) | [TransferResponse](#transferresponse) | Transfer DUSK to another account |
| SendBid | [ConsensusTxRequest](#consensustxrequest) | [TransferResponse](#transferresponse) | Bid a given amount of DUSK, in order to participate in the block generation |
| SendStake | [ConsensusTxRequest](#consensustxrequest) | [TransferResponse](#transferresponse) | Stake a given amount of DUSK, in order to participate as a provisioner |
| AutomateConsensusTxs | [EmptyRequest](#emptyrequest) | [GenericResponse](#GenericResponse) | Allow the node to bid and stake DUSK for you, when necessary, in order to automatically participate in consensus |
| GetWalletStatus | [EmptyRequest](#emptyrequest) | [WalletStatusResponse](#walletstatusresponse) | Query the node on whether or not it has a wallet file loaded |
| GetAddress | [EmptyRequest](#emptyrequest) | [LoadResponse](#loadresponse) | Show the public key of the loaded wallet file |
| GetSyncProgress | [EmptyRequest](#emptyrequest) | [SyncProgressResponse](#syncprogressresponse) | Request the progress of synchronization in percentage form |
| GetBalance | [EmptyRequest](#emptyrequest) | [BalanceResponse](#balanceresponse) | Query the balance of the loaded wallet |
| GetUnconfirmedBalance | [EmptyRequest](#emptyrequest) | [BalanceResponse](#balanceresponse) | Query the unconfirmed balance of the loaded wallet |
| GetTxHistory | [EmptyRequest](#emptyrequest) | [TxHistoryResponse](#txhistoryresponse) | Request a record of all incoming and outgoing transactions for the loaded wallet |
| RebuildChain | [EmptyRequest](#emptyrequest) | [GenericResponse](#genericresponse) | Delete the blockchain, to allow for a full re-sync |
| SelectTx | [SelectRequest](#selectrequest) | [SelectResponse](#selectresponse) | Request an overview of the node's mempool. Can be filtered to only include specific transactions, based on the parameters given |
