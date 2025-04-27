# DB Adapter vs. Legacy HTTP GraphQL – Method Comparison

This document compares each method of the `DatabaseAdapter` trait in
`rusk/src/lib/jsonrpc/infrastructure/db.rs` against its corresponding
GraphQL resolver or logic in the legacy HTTP server (e.g.,
`rusk/src/lib/http/chain/graphql/block.rs`,
`rusk/src/lib/http/chain/graphql/data.rs`).

Methods marked as **(Required)** must be implemented by any type implementing
the `DatabaseAdapter` trait (like `RuskDbAdapter`). Methods marked as
**(Default)** have a default implementation provided by the `DatabaseAdapter`
trait itself, which relies on the required methods.

---

## 1. get_block_by_hash (Required)

### GraphQL Resolver

```rust
1:10:rusk/src/lib/http/chain/graphql/block.rs
pub async fn block_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<Block> {
    let (db, _) = ctx.data::<DBContext>()?;
    // Decode hex and perform a light-block lookup
    let hash = hex::decode(hash)?;
    let header = db.read().await.view(|t| t.light_block(&hash))?;
    Ok(header.map(Block::from))
}
```

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::Block>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;

        let db_client = self.db_client.clone();
        // Uses Ledger::block via spawn_blocking
        let block_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block(&block_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        // Converts node_ledger::Block to model::block::Block
        Ok(block_result.map(model::block::Block::from))
    }
```

### Key Differences

- **Implementation Location**: GraphQL logic is in the HTTP handler; Adapter logic is in the `RuskDbAdapter` implementation of the `DatabaseAdapter` trait.
- **Data Fetch**: GraphQL fetches a `LightBlock` (header + tx IDs); Adapter fetches the full `node_ledger::Block` using `Ledger::block`.
- **Return Type**: GraphQL returns its own `Block` type; Adapter converts the `node_ledger::Block` into the `model::block::Block` type defined for the JSON-RPC layer, which includes derived fields like `transactions_count` and `status` (inferred from the block data structure, not the explicit `Label` in this specific method).
- **Error Handling**: GraphQL uses `FieldError`; Adapter uses `DbError` with specific variants like `InternalError` (for hex decoding issues) and `DbError::from(node::database::error::Error)` for database errors.
- **Concurrency**: Adapter uses `tokio::task::spawn_blocking` to avoid blocking the async runtime during DB access.

---

## 2. get_block_by_height (Default)

### GraphQL Resolver

```rust
1:18:rusk/src/lib/http/chain/graphql/block.rs
pub async fn block_by_height(
    ctx: &Context<'_>,
    height: f64,
) -> OptResult<Block> {
    let (db, _) = ctx.data::<DBContext>()?;
    let block_hash = db.read().await.view(|t| {
        if height >= 0.0 {
            t.block_hash_by_height(height as u64)
        } else {
            Ok(t.op_read(MD_HASH_KEY)?.map(|h| into_array(&h[..])))
        }
    })?;
    if let Some(hash) = block_hash {
        return block_by_hash(ctx, hex::encode(hash)).await;
    }
    Ok(None)
}
```

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::Block>, DbError> {
        match self.get_block_hash_by_height(height).await? { // Uses Required get_block_hash_by_height
            Some(hash) => self.get_block_by_hash(&hash).await, // Uses Required get_block_by_hash
            None => Ok(None),
        }
    }
```

### Key Differences

- **Implementation Location**: GraphQL logic is in the HTTP handler; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **API Signature**: GraphQL accepts `f64` (allowing negative for latest); Adapter uses `u64`. Getting the latest block requires `get_latest_block`.
- **Composition**: GraphQL directly calls `block_hash_by_height` and then `block_by_hash` (the GraphQL resolver). The Adapter's default implementation calls the **trait's own required methods**: `get_block_hash_by_height` and `get_block_by_hash`.
- **Data Fetch**: GraphQL fetches hash then `LightBlock`; Adapter (via defaults) fetches hash, then full `node_ledger::Block`.
- **Error Handling**: GraphQL uses `FieldError`; Adapter uses `DbError`.

---

## 3. get_latest_block (Default)

### GraphQL Resolver

```rust
1:27:rusk/src/lib/http/chain/graphql/block.rs
pub async fn last_block(ctx: &Context<'_>) -> FieldResult<Block> {
    let (db, _) = ctx.data::<DBContext>()?;
    let block = db.read().await.view(|t| {
        let hash = t.op_read(MD_HASH_KEY)?;
        match hash {
            None => Ok(None),
            Some(h) => t.light_block(&h),
        }
    })?;
    block.map(Block::from)
        .ok_or_else(|| FieldError::new("Cannot find last block"))
}
```

### Database Adapter (Default Trait Impl)

```rust
    async fn get_latest_block(&self) -> Result<model::block::Block, DbError> {
        let height = self.get_block_height().await?; // Uses Default get_block_height
        self.get_block_by_height(height).await?.ok_or_else(|| { // Uses Default get_block_by_height
            DbError::NotFound(format!(
                "Latest block not found at height {}",
                height
            ))
        })
    }
```

### Key Differences

- **Implementation Location**: GraphQL logic is in the HTTP handler; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Composition**: GraphQL reads the tip hash from metadata (`MD_HASH_KEY`) and then fetches the `LightBlock`. The Adapter's default implementation calls **other default methods**: `get_block_height` (which reads metadata and gets header) and then `get_block_by_height` (which gets hash then full block).
- **Data Fetch**: GraphQL reads metadata key then `LightBlock`. Adapter (via defaults) reads metadata key -> gets header -> gets hash -> gets full block.
- **Error Handling**: GraphQL returns `FieldError` on missing tip; Adapter's default implementation returns `DbError::NotFound` if intermediate calls fail (e.g., `get_block_height` or `get_block_by_height` return `None`).

---

## 4. get_blocks_range (Default)

### GraphQL Resolver

```rust
86:101:rusk/src/lib/http/chain/graphql/block.rs
pub async fn blocks_range(
    ctx: &Context<'_>,
    from: u64,
    to: u64,
) -> FieldResult<Vec<Block>> {
    let (db, _) = ctx.data::<DBContext>()?;
    let mut blocks = db.read().await.view(|t| {
        let mut blocks = vec![];
        let mut hash_to_search = None;
        for height in (from..=to).rev() {
            if hash_to_search.is_none() {
                hash_to_search = t.block_hash_by_height(height)?;
            }
            if let Some(hash) = hash_to_search {
                let h = t.light_block(&hash)?.expect("Block to be found");
                hash_to_search = h.header.prev_block_hash.into();
                blocks.push(Block::from(h))
            }
        }
        Ok::<_, anyhow::Error>(blocks)
    })?;
    blocks.reverse();
    Ok(blocks)
}
```

### Database Adapter (Default Trait Impl)

```rust
    async fn get_blocks_range(
        &self,
        height_start: u64,
        height_end: u64,
    ) -> Result<Vec<model::block::Block>, DbError> {
        if height_start > height_end {
            return Err(DbError::InternalError(
                "Start height cannot be greater than end height".into(),
            ));
        }
        let futures =
            (height_start..=height_end).map(|h| self.get_block_by_height(h)); // Uses Default get_block_by_height
        let results: Vec<Result<Option<model::block::Block>, DbError>> =
            join_all(futures).await; // Concurrent execution
        // Filters out None results (missing blocks)
        results.into_iter().filter_map(Result::transpose).collect()
    }
```

### Key Differences

- **Implementation Location**: GraphQL logic is in the HTTP handler; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Concurrency**: Adapter fetches blocks **concurrently** using `join_all` by calling `get_block_by_height` for each height. GraphQL iterates **sequentially** in reverse, using `prev_block_hash` lookups.
- **Fetch Logic**: GraphQL uses direct DB calls (`block_hash_by_height`, `light_block`). Adapter's default implementation calls **another default method**: `get_block_by_height` for each height in the range.
- **Block Data**: Adapter fetches full block; GraphQL fetches `LightBlock`.
- **Error Handling**: Adapter returns `DbError::InternalError` for invalid range. It **skips missing blocks** within the range (returns only found blocks), whereas GraphQL uses `expect` (panic) if a block isn't found after finding its hash via `prev_block_hash`.

---

## 5. get_blocks_by_hashes (Default)

### GraphQL Resolver

(No direct equivalent in the legacy GraphQL API for fetching multiple specific blocks by hash simultaneously. The `block_by_hash` resolver handles only a single hash.)

### Database Adapter (Default Trait Impl)

```rust
    async fn get_blocks_by_hashes(
        &self,
        hashes_hex: &[String],
    ) -> Result<Vec<Option<model::block::Block>>, DbError> {
        let futures = hashes_hex.iter().map(|h| self.get_block_by_hash(h)); // Uses Required get_block_by_hash
        try_join_all(futures).await // Use try_join_all to propagate errors
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides batch fetching by hash, which legacy GraphQL lacked.
- **Concurrency**: Adapter fetches blocks **concurrently** using `try_join_all` by calling the required `get_block_by_hash` for each hash.
- **Composition**: Adapter's default implementation calls the **required method**: `get_block_by_hash`.
- **Error Handling**: Adapter returns `Vec<Option<model::block::Block>>`. If `get_block_by_hash` returns `Ok(None)` for a hash, the corresponding element in the result vector is `None`. If any call returns a `DbError`, `try_join_all` propagates the first error encountered.

---

## 6. get_latest_block_header (Default)

### GraphQL Resolver

No specific resolvers for fetching *only* headers existed. Clients queried the `header` field within a `block_by_hash` or `last_block` query, fetching at least a `LightBlock`.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_latest_block_header(
        &self,
    ) -> Result<model::block::BlockHeader, DbError> {
        let block = self.get_latest_block().await?; // Uses Default get_latest_block
        Ok(block.header)
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent for header-only; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Efficiency**: Legacy required fetching at least `LightBlock`. Adapter's default implementation calls `get_latest_block` (which fetches the full block via other defaults), so it's not optimized for header-only retrieval *in the default implementation*. However, the required primitive `get_block_header_by_hash` *is* efficient.
- **Composition**: Adapter's default implementation calls **another default method**: `get_latest_block`.
- **Functionality**: Adapter provides a dedicated conceptual endpoint for the latest header, although the default implementation isn't the most direct path.

---

## 7. get_block_header_by_height (Default)

### GraphQL Resolver

No specific resolvers for fetching *only* headers existed. Clients queried the `header` field within `block_by_height`, fetching a `LightBlock`.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_header_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockHeader>, DbError> {
        match self.get_block_hash_by_height(height).await? { // Uses Required get_block_hash_by_height
            Some(hash) => self.get_block_header_by_hash(&hash).await, // Uses Required get_block_header_by_hash
            None => Ok(None),
        }
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent for header-only; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Efficiency**: Legacy required fetching `LightBlock`. Adapter's default implementation efficiently uses the required primitives `get_block_hash_by_height` and `get_block_header_by_hash` (which directly maps to `Ledger::block_header`).
- **Composition**: Adapter's default implementation calls **two required methods**: `get_block_hash_by_height` and `get_block_header_by_hash`.
- **Functionality**: Adapter provides a dedicated, efficient method for header-by-height retrieval.

---

## 8. get_block_headers_range (Default)

### GraphQL Resolver

No direct equivalent found. Clients likely used `blocks_range` and extracted headers, or made multiple individual queries.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_headers_range(
        &self,
        height_start: u64,
        height_end: u64,
    ) -> Result<Vec<model::block::BlockHeader>, DbError> {
        if height_start > height_end {
            return Err(DbError::InternalError(
                "Start height cannot be greater than end height".into(),
            ));
        }
        let futures = (height_start..=height_end)
            .map(|h| self.get_block_header_by_height(h)); // Uses Default get_block_header_by_height
        let results: Vec<Result<Option<model::block::BlockHeader>, DbError>> =
            join_all(futures).await; // Concurrent execution
        // Filters out None results (missing headers)
        results.into_iter().filter_map(Result::transpose).collect()
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides batch header query by range, not available in legacy.
- **Concurrency**: Adapter fetches headers **concurrently** using `join_all` by calling the default `get_block_header_by_height` for each height.
- **Composition**: Adapter's default implementation calls **another default method**: `get_block_header_by_height`.
- **Missing Headers**: Similar to `get_blocks_range`, it **skips missing headers** (returns only found headers).

---

## 9. get_block_headers_by_hashes (Default)

### GraphQL Resolver

No direct equivalent found. Clients likely made multiple individual queries.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_headers_by_hashes(
        &self,
        hashes_hex: &[String],
    ) -> Result<Vec<Option<model::block::BlockHeader>>, DbError> {
        let futures =
            hashes_hex.iter().map(|h| self.get_block_header_by_hash(h)); // Uses Required get_block_header_by_hash
        try_join_all(futures).await // Concurrent execution, propagates errors
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides batch header query by hashes, not available in legacy.
- **Concurrency**: Adapter fetches headers **concurrently** using `try_join_all` by calling the required `get_block_header_by_hash` for each hash.
- **Composition**: Adapter's default implementation calls the **required method**: `get_block_header_by_hash`.
- **Error Handling**: Returns `Vec<Option<...>>`. Propagates the first `DbError` encountered via `try_join_all`. `Ok(None)` results from the inner call are preserved as `None` in the vector.

---

## 10. get_block_hash_by_height (Required)

### GraphQL Resolver

This functionality was used internally within the `block_by_height` resolver but wasn't exposed as a standalone query.

```rust
1:18:rusk/src/lib/http/chain/graphql/block.rs // Relevant part
    let block_hash = db.read().await.view(|t| {
        if height >= 0.0 {
            t.block_hash_by_height(height as u64)
        } else {
            // ... latest block logic ...
        }
    })?;
```

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn get_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<String>, DbError> {
        let db_client = self.db_client.clone();
        // Uses Ledger::block_hash_by_height via spawn_blocking
        let hash_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block_hash_by_height(height))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        Ok(hash_result.map(hex::encode)) // Encodes to hex string
    }
```

### Key Differences

- **Implementation Location**: Internal in legacy GraphQL; Implemented in `RuskDbAdapter` for the Adapter trait.
- **Functionality**: Adapter exposes this as a dedicated, public method.
- **Implementation**: Directly maps to the underlying `Ledger::block_hash_by_height` DB method via `spawn_blocking`, providing an efficient lookup. Returns hex-encoded string.

---

## 11. get_block_height (Default)

### GraphQL Resolver

No direct equivalent. Clients typically queried `last_block` and extracted the height from the header.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_height(&self) -> Result<u64, DbError> {
        let tip_hash_bytes = self.metadata_op_read(MD_HASH_KEY).await?.ok_or( // Uses Required metadata_op_read
            DbError::NotFound("Tip hash metadata key not found".into()),
        )?;
        let tip_hash: [u8; 32] = tip_hash_bytes.try_into().map_err(|_| {
            DbError::InternalError("Invalid tip hash length in metadata".into())
        })?;
        let header = self
            .get_block_header_by_hash(&hex::encode(tip_hash)) // Uses Required get_block_header_by_hash
            .await?
            .ok_or(DbError::NotFound("Tip block header not found".into()))?;
        Ok(header.height)
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides a direct method for the tip height.
- **Composition**: Adapter's default implementation reads the tip hash from metadata using the **required method** `metadata_op_read(MD_HASH_KEY)`, then fetches the corresponding header using the **required method** `get_block_header_by_hash` to extract the height. This is more efficient than the default `get_latest_block_header`.
- **Efficiency**: More direct than legacy approach of fetching full `last_block`.

---

## 12. get_block_timestamp_by_hash (Default)

### GraphQL Resolver

No direct equivalent. Clients queried `block_by_hash` and extracted the timestamp from the header field.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_timestamp_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<u64>, DbError> {
        Ok(self
            .get_block_header_by_hash(block_hash_hex) // Uses Required get_block_header_by_hash
            .await?
            .map(|h| h.timestamp))
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides a direct method for getting timestamp by hash.
- **Composition**: Adapter's default implementation efficiently calls the **required method** `get_block_header_by_hash` and extracts the `timestamp`.
- **Efficiency**: More efficient than legacy approach.

---

## 13. get_block_timestamp_by_height (Default)

### GraphQL Resolver

No direct equivalent. Clients queried `block_by_height` and extracted the timestamp from the header field.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_timestamp_by_height(
        &self,
        height: u64,
    ) -> Result<Option<u64>, DbError> {
        Ok(self
            .get_block_header_by_height(height) // Uses Default get_block_header_by_height
            .await?
            .map(|h| h.timestamp))
    }
```

### Key Differences

- **Implementation Location**: No direct legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides a direct method for getting timestamp by height.
- **Composition**: Adapter's default implementation calls **another default method**: `get_block_header_by_height` (which efficiently uses required primitives) and extracts the `timestamp`.
- **Efficiency**: More efficient than legacy approach.

---

## 14. get_block_transactions_by_hash (Required)

### GraphQL Resolver

Transactions were typically resolved as a field on the `Block` type. The primary block resolver (`block_by_hash`) fetched a `LightBlock` (containing `txs_id`). A separate field resolver then iterated these IDs and fetched each transaction individually (N+1 pattern).

```rust
// Example: Field resolver within `impl Block` in graphql/data.rs (Conceptual)
    async fn transactions(&self, ctx: &Context<'_>) -> FieldResult<Vec<SpentTransaction>> {
        let db = ctx.data::<DBContext>()?.0.read().await;
        db.view(|t| {
            let mut txs = Vec::with_capacity(self.txs_id.len());
            // self.txs_id obtained from LightBlock in parent resolver
            for id in &self.txs_id {
                // N+1 query pattern
                let tx = t.ledger_tx(id)?.ok_or_else(|| { // Uses Ledger::ledger_tx
                    FieldError::new("Cannot find transaction")
                })?;
                txs.push(SpentTransaction(tx));
            }
            Ok(txs)
        })
    }
```

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn get_block_transactions_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;

        let db_client = self.db_client.clone();
        // Uses Ledger::block via spawn_blocking
        let block_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block(&block_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        match block_result {
            Some(block) => {
                // Extracts txs directly from the full block
                let transactions = block
                    .txs()
                    .iter()
                    .cloned() // Clones node_ledger::Transaction
                    .map(model::transaction::TransactionResponse::from) // Converts to model
                    .collect();
                Ok(Some(transactions))
            }
            None => Ok(None),
        }
    }
```

### Key Differences

- **Implementation Location**: GraphQL logic split between block resolver and transaction field resolver; Adapter logic is in `RuskDbAdapter`.
- **DB Efficiency**: Adapter is much more efficient. It fetches the full `node_ledger::Block` (which includes all `node_ledger::Transaction` structs) in **one DB call** (`Ledger::block`). Legacy GraphQL fetched a `LightBlock` (header + tx IDs) and then made **N separate DB calls** (`Ledger::ledger_tx`) to get each `node_ledger::SpentTransaction`.
- **Data Fetched**: Adapter fetches `node_ledger::Block`. Legacy fetched `LightBlock` + N * `node_ledger::SpentTransaction`.
- **Return Model**: Adapter returns `model::transaction::TransactionResponse`, which wraps `node_ledger::Transaction` and doesn't contain execution status (`gas_spent`, `error`). Legacy field resolver likely returned a type wrapping `node_ledger::SpentTransaction`, which *does* include execution status.

---

## 15. get_block_transactions_by_height (Default)

### GraphQL Resolver

Similar to `by_hash`, clients queried `block_by_height` (getting `LightBlock`), then the `transactions` field resolver (shown in #14) iterated IDs making N `ledger_tx` calls.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_transactions_by_height(
        &self,
        height: u64,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        match self.get_block_hash_by_height(height).await? { // Uses Required get_block_hash_by_height
            Some(hash) => self.get_block_transactions_by_hash(&hash).await, // Uses Required get_block_transactions_by_hash
            None => Ok(None),
        }
    }
```

### Key Differences

- **Implementation Location**: GraphQL logic split between resolvers; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Composition**: GraphQL used `block_hash_by_height` -> `light_block` -> N * `ledger_tx`. Adapter's default implementation calls the **required method** `get_block_hash_by_height` and then the **required method** `get_block_transactions_by_hash`.
- **DB Efficiency**: Adapter (via defaults reusing required methods) is more efficient (`block_hash_by_height` + `block`) than legacy (`block_hash_by_height` -> `light_block` -> N * `ledger_tx`).
- **Data Fetched**: Adapter (via defaults) fetches `node_ledger::Block`. Legacy fetched `LightBlock` + N * `node_ledger::SpentTransaction`.
- **Return Model**: Same as `get_block_transactions_by_hash`: Adapter returns `TransactionResponse` (no execution status), Legacy likely returned `SpentTransaction` (containing execution status).

---

## 16. get_candidate_block_by_hash (Default)

### GraphQL Resolver

No equivalent found. Candidate blocks were likely not exposed via the legacy GraphQL API.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_candidate_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::CandidateBlock>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;
        let candidate_block = self.candidate(&block_hash).await?; // Uses Required candidate
        Ok(candidate_block.map(model::block::CandidateBlock::from)) // Converts node_ledger::Block to model
    }
```

### Key Differences

- **Implementation Location**: No legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides access to candidate blocks, which legacy lacked.
- **Composition**: Adapter's default implementation calls the **required method** `candidate` (which maps to `ConsensusStorage::candidate`).
- **Return Type**: Converts the `node_ledger::Block` into `model::block::CandidateBlock`.

---

## 17. get_latest_candidate_block (Default)

### GraphQL Resolver

No equivalent found.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_latest_candidate_block(
        &self,
    ) -> Result<model::block::CandidateBlock, DbError> {
        let latest_header_bytes =
            self.metadata_op_read(MD_LAST_ITER).await?.ok_or( // Uses Required metadata_op_read
                DbError::NotFound("Last iteration metadata not found".into()),
            )?;
        let latest_header =
            ConsensusHeader::read(&mut latest_header_bytes.as_slice()) // Deserializes header
                .map_err(|e| {
                    DbError::InternalError(format!(
                        "Failed to deserialize header: {}",
                        e
                    ))
                })?;
        let candidate_block = self
            .candidate_by_iteration(&latest_header) // Uses Required candidate_by_iteration
            .await?
            .ok_or_else(|| {
                DbError::NotFound(format!(
                    "Candidate block not found for header: {:?}",
                    latest_header
                ))
            })?;
        Ok(model::block::CandidateBlock::from(candidate_block)) // Converts node_ledger::Block to model
    }
```

### Key Differences

- **Implementation Location**: No legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides access to the latest candidate block (based on `MD_LAST_ITER` metadata), which legacy lacked.
- **Composition**: Adapter's default implementation uses the **required method** `metadata_op_read(MD_LAST_ITER)` to get the latest consensus header, deserializes it, and then uses the **required method** `candidate_by_iteration` to fetch the block.
- **Return Type**: Converts the `node_ledger::Block` into `model::block::CandidateBlock`.

---

## 18. get_validation_result (Default)

### GraphQL Resolver

No equivalent found. Validation results were likely internal consensus state not exposed via legacy GraphQL.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_validation_result(
        &self,
        prev_block_hash_hex: &str,
        round: u64,
        iteration: u8,
    ) -> Result<Option<model::consensus::ValidationResult>, DbError> {
        let prev_block_hash: [u8; 32] = hex::decode(prev_block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!(
                    "Invalid prev block hash hex: {}",
                    e
                ))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid prev block hash length".into())
            })?;
        let header = ConsensusHeader { // Constructs header identifier
            prev_block_hash,
            round,
            iteration,
        };
        let node_result = self.validation_result(&header).await?; // Uses Required validation_result
        Ok(node_result.map(model::consensus::ValidationResult::from)) // Converts node_payload::ValidationResult to model
    }
```

### Key Differences

- **Implementation Location**: No legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides access to specific validation results by consensus header, which legacy lacked.
- **Composition**: Adapter's default implementation constructs the `ConsensusHeader` identifier and calls the **required method** `validation_result` (which maps to `ConsensusStorage::validation_result`).
- **Return Type**: Converts the `node_payload::ValidationResult` into `model::consensus::ValidationResult`.

---

## 19. get_latest_validation_result (Default)

### GraphQL Resolver

No equivalent found.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_latest_validation_result(
        &self,
    ) -> Result<model::consensus::ValidationResult, DbError> {
        let latest_header_bytes =
            self.metadata_op_read(MD_LAST_ITER).await?.ok_or( // Uses Required metadata_op_read
                DbError::NotFound("Last iteration metadata not found".into()),
            )?;
        let latest_header =
            ConsensusHeader::read(&mut latest_header_bytes.as_slice()) // Deserializes header
                .map_err(|e| {
                    DbError::InternalError(format!(
                        "Failed to deserialize header: {}",
                        e
                    ))
                })?;
        let node_result = self
            .validation_result(&latest_header) // Uses Required validation_result
            .await?
            .ok_or_else(|| {
                DbError::NotFound(format!(
                    "Validation result not found for header: {:?}",
                    latest_header
                ))
            })?;
        Ok(model::consensus::ValidationResult::from(node_result)) // Converts to model
    }
```

### Key Differences

- **Implementation Location**: No legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides access to the latest validation result (based on `MD_LAST_ITER` metadata), which legacy lacked.
- **Composition**: Similar to `get_latest_candidate_block`, it uses the **required method** `metadata_op_read(MD_LAST_ITER)` to get the header and then the **required method** `validation_result` to fetch the result.
- **Return Type**: Converts the `node_payload::ValidationResult` into `model::consensus::ValidationResult`.

---

## 20. get_block_faults_by_hash (Required)

### GraphQL Resolver

Likely handled as a field resolver on the `Block` type. Assumed pattern:

1. Parent resolver (`block_by_hash`) fetched `LightBlock`.
2. `faults` field resolver (if it existed) would iterate `LightBlock.faults_ids`, making N calls to fetch individual `Fault` objects by ID (e.g., using a hypothetical `tx.fault_by_id(...)` or similar).

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn get_block_faults_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockFaults>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;

        let db_client = self.db_client.clone();
        // Uses Ledger::block via spawn_blocking
        let block_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block(&block_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        match block_result {
            Some(block) => {
                // Extracts faults directly from the full block
                let faults: Vec<node_ledger::Fault> = block.faults().to_vec();
                // Converts Vec<node_ledger::Fault> to model::block::BlockFaults
                let block_faults = model::block::BlockFaults::try_from(faults)
                    .map_err(|e| {
                        DbError::InternalError(format!(
                            "Failed to convert faults: {}",
                            e
                        ))
                    })?;
                Ok(Some(block_faults))
            }
            None => Ok(None),
        }
    }
```

### Key Differences

- **Implementation Location**: Legacy likely field resolver; Adapter logic is in `RuskDbAdapter`.
- **DB Efficiency**: Adapter is more efficient, fetching faults along with the full `node_ledger::Block` in **one DB call** (`Ledger::block`). Legacy likely used N+1 queries if faults weren't part of `LightBlock`.
- **Data Structure**: Adapter extracts `Vec<node_ledger::Fault>` from the fetched `Block`. Legacy likely fetched `Fault` objects individually or extracted IDs from `LightBlock`.
- **Conversion**: Adapter performs `TryFrom<Vec<node_ledger::Fault>>` to `model::block::BlockFaults`.

---

## 21. get_block_faults_by_height (Default)

### GraphQL Resolver

Likely handled as a field resolver on the `Block` type, similar to `get_block_faults_by_hash` but starting from `block_by_height`. Assumed pattern: `block_by_height` -> `LightBlock` -> N * fault fetches.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_block_faults_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockFaults>, DbError> {
        match self.get_block_hash_by_height(height).await? { // Uses Required get_block_hash_by_height
            Some(hash) => self.get_block_faults_by_hash(&hash).await, // Uses Required get_block_faults_by_hash
            None => Ok(None),
        }
    }
```

### Key Differences

- **Implementation Location**: GraphQL logic split resolvers; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Composition**: GraphQL used `block_hash_by_height` -> `light_block` -> N * fault fetches. Adapter's default implementation calls the **required method** `get_block_hash_by_height` and then the **required method** `get_block_faults_by_hash`.
- **DB Efficiency**: Adapter (via defaults reusing required methods) is more efficient (`block_hash_by_height` + `block`) than legacy (`block_hash_by_height` -> `light_block` -> N * fault fetches).
- **Data Fetched**: Adapter (via defaults) fetches `node_ledger::Block`. Legacy fetched `LightBlock` + N * `Fault` objects.

---

## 22. get_block_label_by_height (Required)

### GraphQL Resolver

No direct equivalent. The concept of a block `Label` (Final, Pending, Stale) was likely not exposed as a distinct queryable field in the legacy API.

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn get_block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockLabel>, DbError> {
        let db_client = self.db_client.clone();
        // Uses Ledger::block_label_by_height via spawn_blocking
        let label_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block_label_by_height(height))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        // Extracts label from (hash, label) pair and converts to model
        Ok(label_result
            .map(|(_hash, label)| model::block::BlockLabel::from(label)))
    }
```

### Key Differences

- **Functionality**: Adapter provides a way to query the consensus status (`Label`) of a block directly, which was likely absent in legacy.
- **Implementation**: Adapter uses the underlying `Ledger::block_label_by_height` DB method via `spawn_blocking` and converts the `node_ledger::Label` to `model::block::BlockLabel`.

---

## 23. get_latest_block_label (Default)

### GraphQL Resolver

No equivalent found.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_latest_block_label(
        &self,
    ) -> Result<model::block::BlockLabel, DbError> {
        let height = self.get_block_height().await?; // Uses Default get_block_height
        self.get_block_label_by_height(height) // Uses Required get_block_label_by_height
            .await?
            .ok_or_else(|| {
                DbError::NotFound(format!(
                    "Label not found for latest block {}",
                    height
                ))
            })
    }
```

### Key Differences

- **Implementation Location**: No legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides a direct way to get the status (`Label`) of the current chain tip.
- **Composition**: Adapter's default implementation calls the **default method** `get_block_height` and then the **required method** `get_block_label_by_height`.
- **Error Handling**: Returns `DbError::NotFound` if the label for the determined latest height is not found.

---

## 24. get_spent_transaction_by_hash (Required)

### GraphQL Resolver

This likely corresponds to the core logic used by the transaction field resolver (see #14) or a dedicated `transaction_by_hash` resolver if one existed. It would use `Ledger::ledger_tx`.

```rust
// Conceptual legacy resolver using Ledger::ledger_tx
pub async fn transaction_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<SpentTransaction> { // Assuming SpentTransaction wraps node_ledger::SpentTransaction
    let (db, _) = ctx.data::<DBContext>()?;
    let tx_id = hex::decode(hash)?;
    let tx = db.read().await.view(|t| t.ledger_tx(&tx_id))?;
    Ok(tx.map(SpentTransaction))
}
```

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn get_spent_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
    ) -> Result<Option<node_ledger::SpentTransaction>, DbError> {
        let tx_hash: [u8; 32] = hex::decode(tx_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid tx hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid tx hash length".into())
            })?;

        let db_client = self.db_client.clone();
        // Uses Ledger::ledger_tx via spawn_blocking
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.ledger_tx(&tx_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }
```

### Key Differences

- **Implementation Location**: Legacy likely in a field resolver or dedicated resolver; Adapter logic is in `RuskDbAdapter`.
- **Return Type**: Legacy likely wrapped the result in a GraphQL-specific type; Adapter **returns the raw `node_ledger::SpentTransaction`** directly. This is a primitive method; conversion to `model::transaction::TransactionInfo` happens in the default `get_transaction_by_hash` method.
- **Implementation**: Both use `Ledger::ledger_tx`. Adapter uses `spawn_blocking`.

---

## 25. get_transaction_by_hash (Default)

### GraphQL Resolver

This combines fetching the `SpentTransaction` (as in #24) with potentially fetching block header info and finding the transaction index within the block.

```rust
// Conceptual legacy logic combining multiple lookups
pub async fn transaction_info_by_hash(
    ctx: &Context<'_>,
    hash: String,
    include_tx_index: bool,
) -> OptResult<TransactionInfo> { // Assuming a TransactionInfo type
    let (db, _) = ctx.data::<DBContext>()?;
    let db_read = db.read().await;

    // 1. Get SpentTransaction
    let tx_id = hex::decode(&hash)?;
    let spent_tx = db_read.view(|t| t.ledger_tx(&tx_id))?;
    let spent_tx = match spent_tx {
        Some(tx) => tx,
        None => return Ok(None),
    };

    // 2. Get Block Header for timestamp/block hash
    let block_hash_bytes = db_read.view(|t| t.block_hash_by_height(spent_tx.block_height))?;
    let block_header = match block_hash_bytes {
        Some(bh) => db_read.view(|t| t.block_header(&bh))?,
        None => None, // Handle case where header is missing?
    };
    let (block_hash_hex, timestamp) = block_header.map_or((hash.clone(), 0), |h| (hex::encode(h.hash), h.header.timestamp));

    // 3. Get Tx Index (if requested)
    let mut tx_index = None;
    if include_tx_index {
        if let Some(bh) = block_hash_bytes {
            // Fetch LightBlock or full block to get tx list
            let block_txs = db_read.view(|t| t.light_block(&bh))?.map(|lb| lb.txs_id); // Or full block
            if let Some(ids) = block_txs {
                 tx_index = ids.iter().position(|id| id == &tx_id).map(|i| i as u32);
            }
        }
    }

    // 4. Construct TransactionInfo
    Ok(Some(TransactionInfo { /* fields populated from spent_tx, block_hash_hex, timestamp, tx_index */ }))
}

```

### Database Adapter (Default Trait Impl)

```rust
    async fn get_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
        include_tx_index: bool,
    ) -> Result<Option<model::transaction::TransactionInfo>, DbError> {
        // 1. Uses Required get_spent_transaction_by_hash
        if let Some(spent_tx) =
            self.get_spent_transaction_by_hash(tx_hash_hex).await?
        {
            // 2. Uses Default get_block_header_by_height
            let header_opt = self
                .get_block_header_by_height(spent_tx.block_height)
                .await?;
            let (block_hash, timestamp) = header_opt
                .map_or((None, None), |h| {
                    (h.hash, h.timestamp)
                });

            // 3. Uses Required get_block_transactions_by_hash (if needed)
            let mut tx_index = None;
            if include_tx_index && block_hash != tx_hash_hex { // Avoid fetch if hash matches (genesis?)
                if let Some(txs) =
                    self.get_block_transactions_by_hash(&block_hash).await?
                {
                    tx_index = txs
                        .iter()
                        .position(|tx| tx.base.tx_hash == tx_hash_hex)
                        .map(|i| i as u32);
                }
            }

            // 4. Construct model::transaction::TransactionInfo
            let response = model::transaction::TransactionResponse::from(
                spent_tx.inner.clone(),
            ); // Convert base tx

            Ok(Some(model::transaction::TransactionInfo {
                base: response.base,
                transaction_data: response.transaction_data,
                block_height: spent_tx.block_height,
                block_hash,
                tx_index, // Option<u32>
                gas_spent: spent_tx.gas_spent,
                timestamp,
                error: spent_tx.err,
            }))
        } else {
            Ok(None) // Transaction not found in ledger
        }
    }
```

### Key Differences

- **Implementation Location**: Legacy logic likely spread across resolvers/helpers; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Composition**: Legacy uses direct DB calls. Adapter's default implementation combines calls to multiple **required methods**:
  - `get_spent_transaction_by_hash` (Required)
  - `get_block_header_by_height` (Default)
  - `get_block_transactions_by_hash` (Required, only if `include_tx_index` is true and block context is needed)
- **Data Fetch**: Both fetch similar underlying data (spent tx, header, potentially block txs). Adapter leverages the efficiency of `get_block_transactions_by_hash` (one call for all txs) vs legacy potentially fetching `LightBlock`.
- **Return Type**: Adapter returns the specific `model::transaction::TransactionInfo` struct, which includes execution status (`gas_spent`, `error`) from the `SpentTransaction` and contextual info like `block_hash`, `timestamp`, and optional `tx_index`.

---

## 26. get_transaction_status (Default)

### GraphQL Resolver

No direct equivalent for a combined "status" endpoint. Clients would need to:

1. Check if the transaction exists in the ledger (`ledger_tx_exists`).
2. If yes, potentially fetch `SpentTransaction` (`ledger_tx`) to see if `err` is set (Failed vs Executed) and get block info.
3. If no, check if it exists in the mempool (`mempool_tx_exists`).
4. If neither, assume NotFound.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_transaction_status(
        &self,
        tx_hash_hex: &str,
    ) -> Result<model::transaction::TransactionStatus, DbError> {
        let tx_id: [u8; 32] = hex::decode(tx_hash_hex)
            // ... hex decode error handling ...
            .map_err(|_| {
                DbError::InternalError("Invalid tx hash length".into())
            })?;

        // 1. Check Ledger
        if self.ledger_tx_exists(&tx_id).await? { // Uses Required ledger_tx_exists
            // Fetch details if confirmed
            match self.get_spent_transaction_by_hash(tx_hash_hex).await? { // Uses Required get_spent_transaction_by_hash
                Some(spent_tx) => {
                    // Get block context
                    let header_opt = self
                        .get_block_header_by_height(spent_tx.block_height) // Uses Default get_block_header_by_height
                        .await?;
                    let (block_hash, timestamp) = header_opt
                        .map_or((None, None), |h| {
                            (Some(h.hash), Some(h.timestamp))
                        });

                    // Determine status (Failed vs Executed)
                    let status_type = if spent_tx.err.is_some() {
                        model::transaction::TransactionStatusType::Failed
                    } else {
                        model::transaction::TransactionStatusType::Executed
                    };

                    // Construct Status model
                    Ok(model::transaction::TransactionStatus {
                        status: status_type,
                        block_height: Some(spent_tx.block_height),
                        block_hash,
                        gas_spent: Some(spent_tx.gas_spent),
                        timestamp,
                        error: spent_tx.err,
                    })
                }
                // Should ideally not happen if ledger_tx_exists was true
                None => Err(DbError::InternalError(format!(
                    "Tx {} exists in ledger but SpentTransaction not found",
                    tx_hash_hex
                ))),
            }
        // 2. Check Mempool
        } else if self.mempool_tx_exists(tx_id).await? { // Uses Required mempool_tx_exists
            Ok(model::transaction::TransactionStatus {
                status: model::transaction::TransactionStatusType::Pending,
                block_height: None,
                block_hash: None,
                gas_spent: None,
                timestamp: None,
                error: None,
            })
        // 3. Not Found
        } else {
            // Current impl returns Error, could return Status::NotFound instead
            Err(DbError::NotFound(format!(
                "Transaction {} not found",
                tx_hash_hex
            )))
        }
    }
```

### Key Differences

- **Implementation Location**: Legacy requires multiple client-side checks; Adapter provides a single **default implementation** in the trait.
- **Functionality**: Adapter provides a convenient method to get a comprehensive status (Pending, Executed, Failed, or NotFound error).
- **Composition**: Adapter's default implementation orchestrates calls to multiple **required methods**: `ledger_tx_exists`, `get_spent_transaction_by_hash`, `mempool_tx_exists` and one **default method**: `get_block_header_by_height`.
- **Return Type**: Adapter returns `model::transaction::TransactionStatus`, consolidating information relevant to the status. Legacy would return different types/errors depending on the check performed.
- **Error Handling**: Adapter currently returns `DbError::NotFound` if the transaction is not found in either ledger or mempool. An alternative design could return `Ok(TransactionStatus { status: NotFound, .. })`.

---

## 27. ledger_tx_exists (Required)

### GraphQL Resolver

Likely used internally, or potentially exposed if needed. Would map to `Ledger::ledger_tx_exists`.

```rust
// Conceptual legacy usage
async fn check_tx_confirmed(ctx: &Context<'_>, hash: String) -> FieldResult<bool> {
    let (db, _) = ctx.data::<DBContext>()?;
    let tx_id = hex::decode(hash)?;
    let exists = db.read().await.view(|t| t.ledger_tx_exists(&tx_id))?;
    Ok(exists)
}
```

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn ledger_tx_exists(
        &self,
        tx_id: &[u8; 32],
    ) -> Result<bool, DbError> {
        let tx_id_copy = *tx_id;
        let db_client = self.db_client.clone();
        // Uses Ledger::ledger_tx_exists via spawn_blocking
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.ledger_tx_exists(&tx_id_copy[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }
```

### Key Differences

- **Implementation Location**: Legacy internal/helper; Adapter logic in `RuskDbAdapter`.
- **Implementation**: Both map directly to `Ledger::ledger_tx_exists`. Adapter uses `spawn_blocking`.

---

## 28. mempool_tx (Required)

### GraphQL Resolver

No direct equivalent found for fetching a specific mempool transaction in the provided legacy snippets.

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn mempool_tx(
        &self,
        tx_id: [u8; 32],
    ) -> Result<Option<node_ledger::Transaction>, DbError> {
        let db_client = self.db_client.clone();
        // Uses Mempool::mempool_tx via spawn_blocking
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.mempool_tx(tx_id))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }
```

### Key Differences

- **Functionality**: Adapter provides direct access to a specific mempool transaction by ID; legacy equivalent likely missing.
- **Implementation**: Adapter maps directly to `Mempool::mempool_tx` via `spawn_blocking`.
- **Return Type**: Returns the raw `node_ledger::Transaction`. Conversion to `model::transaction::TransactionResponse` happens in the default `get_mempool_transaction_by_hash`.

---

## 29. mempool_tx_exists (Required)

### GraphQL Resolver

No direct equivalent found, but conceptually simple check using `Mempool::mempool_tx_exists`.

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn mempool_tx_exists(
        &self,
        tx_id: [u8; 32],
    ) -> Result<bool, DbError> {
        let db_client = self.db_client.clone();
        // Uses Mempool::mempool_tx_exists via spawn_blocking
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.mempool_tx_exists(tx_id))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }
```

### Key Differences

- **Functionality**: Adapter provides direct check for mempool existence; legacy equivalent likely missing or internal.
- **Implementation**: Adapter maps directly to `Mempool::mempool_tx_exists` via `spawn_blocking`.

---

## 30. get_mempool_info (Default)

### GraphQL Resolver

No direct equivalent for aggregated mempool info (count, min/max fee) found. Clients would need multiple calls (e.g., count + iterators for fees).

### Database Adapter (Default Trait Impl)

```rust
    async fn get_mempool_info(
        &self,
    ) -> Result<model::mempool::MempoolInfo, DbError> {
        // Execute the three required method calls concurrently
        let (count_res, fee_high_res, fee_low_res) = try_join!(
            self.mempool_txs_count(),
            self.mempool_txs_ids_sorted_by_fee(),
            self.mempool_txs_ids_sorted_by_low_fee()
        )?;

        // Process the results
        let count = count_res as u64;
        let max_fee = fee_high_res.first().map(|(fee, _)| *fee);
        let min_fee = fee_low_res.first().map(|(fee, _)| *fee);

        Ok(model::mempool::MempoolInfo {
            count,
            max_fee,
            min_fee,
        })
    }
```

### Key Differences

- **Implementation Location**: No legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter provides a convenient method for common mempool statistics.
- **Composition**: Adapter's default implementation combines calls to three **required methods**: `mempool_txs_count`, `mempool_txs_ids_sorted_by_fee`, and `mempool_txs_ids_sorted_by_low_fee`. This implementation uses `try_join` to execute the three required method calls concurrently.
- **Return Type**: Returns the aggregated `model::mempool::MempoolInfo`.

---

## 31. get_gas_price (Default)

### GraphQL Resolver

No equivalent found for calculating gas price statistics (min, max, median, average) from the mempool.

### Database Adapter (Default Trait Impl)

```rust
    async fn get_gas_price(
        &self,
        max_transactions: Option<usize>,
    ) -> Result<model::gas::GasPriceStats, DbError> {
        // 1. Uses Required mempool_txs_ids_sorted_by_fee
        let mut prices_ids = self.mempool_txs_ids_sorted_by_fee().await?;

        // 2. Optional truncation
        if let Some(max) = max_transactions {
            prices_ids.truncate(max);
        }

        let gas_prices: Vec<u64> = prices_ids.into_iter().map(|(p, _)| p).collect();

        // 3. Calculate stats (avg, min, max, median)
        if gas_prices.is_empty() {
            // Default values if no transactions considered
            Ok(GasPriceStats { average: 1, max: 1, median: 1, min: 1 })
        } else {
            let count = gas_prices.len() as u64;
            let sum: u64 = gas_prices.iter().sum();
            let average = (sum + count - 1) / count; // Ceiling division
            let max = *gas_prices.first().unwrap_or(&1); // Sorted desc
            let min = *gas_prices.last().unwrap_or(&1);

            let mut sorted_prices = gas_prices; // Clone/sort asc for median
            sorted_prices.sort_unstable();
            let mid = sorted_prices.len() / 2;
            let median = if sorted_prices.len() % 2 == 0 {
                (sorted_prices[mid - 1] + sorted_prices[mid]) / 2
            } else {
                sorted_prices[mid]
            };

            Ok(GasPriceStats { average, max, median, min })
        }
    }
```

### Key Differences

- **Implementation Location**: No legacy equivalent; Adapter logic is a **default implementation** within the `DatabaseAdapter` trait.
- **Functionality**: Adapter calculates gas price statistics based on fees of (optionally top N) transactions in the mempool.
- **Composition**: Adapter's default implementation calls the **required method** `mempool_txs_ids_sorted_by_fee`, then performs calculations (average, min, max, median) on the collected fees.
- **Return Type**: Returns the calculated `model::gas::GasPriceStats`.

---

## 32. metadata_op_read (Required)

### GraphQL Resolver

Used internally in legacy code (e.g., `last_block` uses `t.op_read(MD_HASH_KEY)`), but likely not exposed as a direct public resolver.

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn metadata_op_read(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, DbError> {
        let key_copy = key.to_vec();
        let db_client = self.db_client.clone();
        // Uses Metadata::op_read via spawn_blocking
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.op_read(&key_copy))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }
```

### Key Differences

- **Functionality**: Adapter exposes raw metadata read capability as a required primitive; legacy likely used it internally only.
- **Implementation**: Adapter maps directly to `Metadata::op_read` via `spawn_blocking`.
- **Return Type**: Returns raw `Option<Vec<u8>>`.

---

## 33. metadata_op_write (Required)

### GraphQL Resolver

Write operations were generally not exposed via the read-only GraphQL API.

### Database Adapter (`RuskDbAdapter` Impl)

```rust
    async fn metadata_op_write(
        &mut self, // Note: requires &mut self
        key: &[u8],
        value: &[u8],
    ) -> Result<(), DbError> {
        let key_copy = key.to_vec();
        let value_copy = value.to_vec();
        let db_client = self.db_client.clone();
        // Uses Metadata::op_write via spawn_blocking with blocking_write
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_write(); // Acquires write lock
            db.update(|v| v.op_write(&key_copy, &value_copy))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }
```

### Key Differences

- **Functionality**: Adapter exposes raw metadata write capability as a required primitive (requiring `&mut self`), primarily for internal node operations or potential future admin APIs. Legacy GraphQL is read-only.
- **Implementation**: Adapter maps directly to `Metadata::op_write` via `spawn_blocking` using `blocking_write()` and `db.update()`.

---

*Note: Other required primitive methods (`candidate`, `candidate_by_iteration`, `validation_result`, `mempool_txs_sorted_by_fee`, `mempool_txs_count`, `mempool_txs_ids_sorted_by_fee`, `mempool_txs_ids_sorted_by_low_fee`) and remaining default methods (`get_transactions_batch`, `get_mempool_transactions`, `get_mempool_transaction_by_hash`, `get_mempool_transactions_count`, `get_chain_stats`) generally follow similar patterns: Required methods map directly to underlying DB traits (`ConsensusStorage`, `Mempool`) via `spawn_blocking`, and default methods compose calls to required/other default methods.*

---
