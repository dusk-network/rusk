 # Node RUES events

### Event Target: Transactions

#### `/on/transactions:#tx_hash/accepted`
- Description: Subscribe for an event of a transaction accepted in a block
- Example:

```
/on/transactions:fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08/accepted
```

- Event  

```json
{
   "hash": "fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08",
   "block_height": 120033,
   "finalized": false,
   "timestamp": 1720770912,
   "gas_spent": 11111111
   "error": ""
}
```

#### `/on/transactions:#tx_hash/finalized`
- Description: Subscribe for an event of a transaction accepted in a FINALIZED block
- Example:

```
/on/transactions:fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08/finalized
```

- Event 

```json
{
   "hash": "fe2ffdcdd27be82fc850bc269ff87fb8e72cf0cd4368e49fe947afeb48167f08",
   "block_height": 120033,
   "finalized": true,
   "timestamp": 1720770912,
   "gas_spent": 11111111
   "error": ""
}
```

### Event Target: Blocks

#### `/on/block:{any,attested,accepted,final}/accepted`
- Description: Subscribe for an event of {any, accepted, attested, confirmed, final} block accepted 
- Example:

```
# Suitable in subscribing for finalized blocks
/on/blocks:final/accepted
```

- Event
```json

{
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

#### `/on/block/reverted`
- Description: Subscribe for an event of a block reverted.
- Example:

```
# Suitable in subscribing for blocks reverted due to fallback execution
/on/block/reverted
```

- Event

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

### Event Target: Mempool

#### `/on/mempool/accepted`
- Description: Subscribe for an event of a transaction inclusion in mempool.
- Event

```json
{
   "tx_hash": "51270ac704f0b59eeb347dfec6bc268644524ae98d7cbf16b8423bb5c824abe9",
   "timestamp": 1720770912,
}
```


#### `/on/mempool/removed`
- Description: Subscribe for an event of a transaction deleted from mempool.
- Event

```json
{
   "tx_hash": "51270ac704f0b59eeb347dfec6bc268644524ae98d7cbf16b8423bb5c824abe9",
   "timestamp": 1720770912,
}
```

### Event Target: Consensus state

#### `/on/consensus/no_candidate`
- Description: Subscribe for an event of a reaching quorum on NoCandidate votes.
- Event

```json
{
   "round": "1234",
   "iteration": 0,
   "generator_bs58": "rvXLHF8DBNwzZ63uSWPki3y7uNgGbdRCrKpouEP9N7awiGBDaP1uz.."
}
```
 
#### `/on/consensus/invalid_candidate`
- Description: Subscribe for an event of a reaching quorum on Invalid votes.
- Event

```json
{
   "round": "1234",
   "iteration": 0,
   "generator_bs58": "rvXLHF8DBNwzZ63uSWPki3y7uNgGbdRCrKpouEP9N7awiGBDaP1uz.."
}
```
 