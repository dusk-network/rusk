 # Node RUES events

### Event Target: Transactions

`/on/transactions:#tx_hash/accepted` 
- Description: Subscribe for an event of a transaction accepted in a non-final block 
- Example: `/on/transactions:fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08/accepted`
- Example: `/on/transactions/accepted`
- HTTP Response Header

```json
{
    "Content-location": "/on/transactions:fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08/accepted"
    "Content-type": "application/json"
}
```

- HTTP Response Payload

```json
{
   "block_height": 120033,
   "block_label": "accepted",
   "timestamp": 1720770912,
   "gas_spent": 11111111
   "error": ""
   // TBD
}
```

#### `/on/transactions:#tx_hash/finalized`
- Description: Subscribe for an event of a transaction accepted in a FINALIZED block
- Example: see also `/on/transactions:#tx_hash/accepted`


#### `/on/transactions:#tx_hash/included`
- Description: Subscribe for an event of a transaction included in mempool
- Example (specified): `/on/transactions:fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08/included`
- Example (any txn): `/on/transactions/included`
- HTTP Response Header
```json
{
    "Content-location": "/on/transactions:fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08/included"
    "Content-type": "application/json"
}
```
- HTTP Response Payload 
```json
{
   "timestamp": 1720770912,
   "error": ""
}
```

### Event Target: Blocks

#### Blocks Accepted
- `/on/blocks:#hash/accepted` - Subscribe for an event of block accepted
- `/on/blocks:#hash/attested` - Subscribe for an event of block attested
- `/on/blocks:#hash/confirmed` - Subscribe for an event of block confirmed 
- `/on/blocks:#hash/finalized` - Subscribe for an event of a finalized block accepted
- Example: `/on/blocks:488a0419e03013a48823f017be790d01d39557478670c9a173099a06da3d739e/accepted`
- Example: `/on/blocks/finalized`
- HTTP Headers
```json
{
    "Content-location": "/on/blocks:488a0419e03013a48823f017be790d01d39557478670c9a173099a06da3d739e/finalized"
    "Content-type": "application/json"
}
```

- HTTP Response Payload
```json
{
  "block": {
    "header": {
      "gasLimit": 5000000000,
      "hash": "488a0419e03013a48823f017be790d01d39557478670c9a173099a06da3d739e",
      "height": 1963,
      "iteration": 0,
      "prevBlockHash": "619e835b9bded6fc4ed39fd5dd9cedf9a61b3e9c54ee1b15b8067766b3215e52",
      "seed": "afa035b209e5f12080f671dcad2cee56679e6d8689e91776abb4487dc4e72cc1b21c0c044869a7ab87a60ed050e9fb6c",
      "stateHash": "ccdc3b146fd05ddb872fe3531253bc9f327ee55063e812c7c09b5166be6978d7",
      "timestamp": 1720792561,
      "txRoot": "0000000000000000000000000000000000000000000000000000000000000000",
      "version": 0
      "label": "final"
    },
    "transactions": []
  }
}

```

#### `/on/blocks:#hash/reverted`
- Description: Subscribe for an event of a block reverted.
- HTTP Response Header
```json
{
    "Content-location": "/on/blocks:488a0419e03013a48823f017be790d01d39557478670c9a173099a06da3d739e/reverted"
    "Content-type": "application/json"
}
```
- HTTP Response Payload

```json
{
  "reason": "fallback", 
   "block": {
    "header": {
      "height": 222,
      "label": "final",
      "prevBlockHash": "1118db0a92c41a8d1e96ed8b0b4c1a49751ac5f7854298a7d53eba7ebfe07caa"
    },
    "txs": []
  }
}
```

### Event Target: Candidates

#### `/on/candidates:#pubkey/missed`
- Description: Subscribe for an event of a reaching quorum on NoCandidate votes.
```json
{
   "round": "1234",
   "iteration": 0,
   // Block generator pubkey
   "pubkey": "rvXLHF8DBNwzZ63uSWPki3y7uNgGbdRCrKpouEP9N7awiGBDaP1uz.."
}
```
 
#### `/on/candidates:#pubkey/invalid`
- Description: Subscribe for an event of a reaching quorum on Invalid votes.
- Event

```json
{
   "round": "1234",
   "iteration": 0,
   // Block generator pubkey
   "pubkey": "rvXLHF8DBNwzZ63uSWPki3y7uNgGbdRCrKpouEP9N7awiGBDaP1uz.."
}
```

### Event Target: Provisioners

#### `/on/provisioners:#pubkey/added`
- Description: Subscribe for an event of an eligible provisioner joining consensus.
- Event

```json
{
   "round": "1234",
   "pubkey": "rvXLHF8DBNwzZ63uSWPki3y7uNgGbdRCrKpouEP9N7awiGBDaP1uz.."
}
```
 
#### `/on/provisioners:#pubkey/slashed`
- Description: Subscribe for an event of a provisioner being slashed.
- Event

```json
{
   "round": "1234",
   "pubkey": "rvXLHF8DBNwzZ63uSWPki3y7uNgGbdRCrKpouEP9N7awiGBDaP1uz..",
   "reason": "missing_candidate",
   "slash_type": "soft",
   "slash_amount": 12222222
}
```

#### `/on/provisioners:#pubkey/rewarded`
- Description: Subscribe for an event of a specified provisioner being rewarded with either Generator or Voter reward.
- Event

```json
{
   "round": "1234",
   "pubkey": "rvXLHF8DBNwzZ63uSWPki3y7uNgGbdRCrKpouEP9N7awiGBDaP1uz..",
   "reward_dusk": 1600000000
}
```