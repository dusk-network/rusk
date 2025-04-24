# VM Adapter vs. Legacy HTTP Server – Method Comparison

This document compares each `VmAdapter` method in `rusk/src/lib/jsonrpc/infrastructure/vm.rs` against its counterpart (or lack thereof) in the legacy HTTP implementation (`rusk/src/lib/http/chain.rs`), with code snippets and detailed explanations of the key differences.

---

## 1. simulate_transaction

### Legacy HTTP Implementation (`rusk/src/lib/http/chain.rs`)

```rust
async fn simulate_tx(&self, tx: &[u8]) -> anyhow::Result<ResponseData> {
    let tx = ProtocolTransaction::from_slice(tx)
        .map_err(|e| anyhow::anyhow!("Invalid transaction: {e:?}"))?;
    // Retrieve VM handler and check gas limit
    let (config, mut session) = {
        let vm_handler = self.inner().vm_handler().read().await;
        if tx.gas_limit() > vm_handler.get_block_gas_limit() {
            return Err(anyhow::anyhow!("Gas limit is too high."));
        }
        // Load latest tip from database
        let tip = load_tip(&self.db()).await?  
            .ok_or_else(|| anyhow::anyhow!("Could not find the tip"))?;
        let height = tip.header.height;
        // Build execution config and open a new block session
        let config = vm_handler.vm_config.to_execution_config(height);
        let session = vm_handler
            .new_block_session(height, vm_handler.tip.read().current)
            .map_err(|e| anyhow::anyhow!("Failed to initialize a session: {e}"))?;
        (config, session)
    };
    // Execute transaction and format response
    let receipt = execute(&mut session, &tx, &config);
    let resp = match receipt {
        Ok(receipt) => json!({
            "gas-spent": receipt.gas_spent,
            "error": receipt.data.err().map(|e| format!("{e:?}")),
        }),
        Err(err) => json!({
            "gas-spent": 0,
            "error": format!("{err:?}")
        }),
    };
    Ok(ResponseData::new(resp))
}
```

### JSON-RPC VM Adapter (`rusk/src/lib/jsonrpc/infrastructure/vm.rs`)

```rust
async fn simulate_transaction(
    &self,
    tx_bytes: Vec<u8>,
) -> Result<SimulationResult, VmError> {
    // Deserialize transaction bytes
    let tx = Transaction::read(&mut tx_bytes.as_slice())
        .map_err(|e| VmError::QueryFailed(format!(
            "Failed to deserialize transaction: {}", e
        )))?;
    // Clone node and determine current state root
    let node = Arc::clone(&self.node_rusk);
    let base_commit = node.state_root();
    // Execute in a blocking task using a dummy height
    let result = tokio::task::spawn_blocking(move || {
        let mut session = node
            .new_block_session(0, base_commit)
            .map_err(|e| VmError::ExecutionFailed(e.to_string()))?;
        let config = node.vm_config.to_execution_config(0);
        let receipt = execute(&mut session, &tx.inner, &config);
        // Map to SimulationResult
        Ok(match receipt {
            Ok(r) => SimulationResult {
                success: true,
                gas_estimate: Some(r.gas_spent),
                error: None,
            },
            Err(err) => SimulationResult {
                success: false,
                gas_estimate: None,
                error: Some(format!("{:?}", err)),
            },
        })
    })
    .await
    .map_err(|e| VmError::InternalError(e.to_string()))?;
    result
}
```

### Key Differences

- **Tip & Height**: The HTTP method loads the on-chain tip from the database and uses its block height; the RPC adapter uses a fixed height of `0`, isolating pure VM logic from on-chain context.
- **Mempool Checks**: HTTP enforces `tx.gas_limit() <= get_block_gas_limit()` upstream; RPC adapter performs no gas-limit guard beyond what the VM itself enforces.
- **State Isolation**: RPC adapter does not alter persistent state, using `state_root()` directly; HTTP's session runs against the node's live state including any uncommitted changes.

---

## 2. preverify_transaction

### Legacy HTTP Implementation (`rusk/src/lib/http/chain.rs`)

```rust
async fn handle_preverify(
    &self,
    data: &[u8],
) -> anyhow::Result<ResponseData> {
    let tx = dusk_core::transfer::Transaction::from_slice(data)
        .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?;
    let db = self.inner().database();
    let vm = self.inner().vm_handler();
    let tx = tx.into();
    // MempoolSrv combines DB and VM preverify logic
    MempoolSrv::check_tx(&db, &vm, &tx, true, usize::MAX)
        .await
        .map_err(|e| e)?;
    Ok(ResponseData::new(DataType::None))
}
```

### JSON-RPC VM Adapter (`rusk/src/lib/jsonrpc/infrastructure/vm.rs`)

```rust
async fn preverify_transaction(
    &self,
    tx_bytes: Vec<u8>,
) -> Result<PreverificationResult, VmError> {
    // Deserialize transaction
    let tx = Transaction::read(&mut tx_bytes.as_slice())
        .map_err(|e| VmError::QueryFailed(format!(
            "Failed to deserialize transaction: {}", e
        )))?;
    // Clone node for blocking call
    let node = Arc::clone(&self.node_rusk);
    // Perform VM-only preverification
    let result = tokio::task::spawn_blocking(move || {
        node.preverify(&tx)
            .map_err(|e| VmError::QueryFailed(e.to_string()))
    })
    .await
    .map_err(|e| VmError::InternalError(e.to_string()))?;
    result
}
```

### Key Differences

- **Full Mempool vs. VM-Only**: HTTP uses `MempoolSrv::check_tx`, which combines database nonce checks, fee balance checks, and VM signature/nullifier validation. RPC adapter calls `VMExecution::preverify` directly, focusing solely on VM-layer preverification.

---

## 3. get_state_root

### Legacy HTTP

No dedicated endpoint for state root; internal VM queries use `session.root()` but HTTP does not expose it.

### JSON-RPC VM Adapter (`rusk/src/lib/jsonrpc/infrastructure/vm.rs`)

```rust
async fn get_state_root(&self) -> Result<[u8; 32], VmError> {
    Ok(self.node_rusk.state_root())
}
```

### Explanation

The adapter exposes the raw 32-byte state root, a feature not present in legacy HTTP.

---

## 4. get_block_gas_limit

### Legacy HTTP

HTTP's simulate path reads `vm_handler.get_block_gas_limit()` to guard against oversize transactions, but does not expose it directly.

### JSON-RPC VM Adapter (`rusk/src/lib/jsonrpc/infrastructure/vm.rs`)

```rust
async fn get_block_gas_limit(&self) -> Result<u64, VmError> {
    Ok(self.node_rusk.vm_config.block_gas_limit)
}
```

### Explanation

Provides direct access to the VM's configured block gas limit, enabling clients to retrieve it via JSON-RPC.

---

## 5. get_provisioners & Stake Queries

### Legacy HTTP

No direct HTTP endpoint; GraphQL might surface provisioners in specialized queries, but no raw RPC method exists.

### JSON-RPC VM Adapter

```rust
async fn get_provisioners(&self) -> Result<Provisioners, VmError> {
    let node = Arc::clone(&self.node_rusk);
    let base_commit = node.state_root();
    let result = tokio::task::spawn_blocking(move || {
        node.get_provisioners(base_commit)
            .map_err(|e| VmError::QueryFailed(e.to_string()))
    })
    .await
    .map_err(|e| VmError::InternalError(e.to_string()))?;
    result
}

async fn get_stake_info_by_pk(&self, pk: &BlsPublicKey) -> Result<Option<Stake>, VmError> {
    let key = pk.clone();
    let node = Arc::clone(&self.node_rusk);
    let result = tokio::task::spawn_blocking(move || {
        node.get_provisioner(&key)
            .map_err(|e| VmError::QueryFailed(e.to_string()))
    })
    .await
    .map_err(|e| VmError::InternalError(e.to_string()))?;
    result
}

async fn get_all_stake_data(&self) -> Result<Vec<(BlsPublicKey, Stake)>, VmError> {
    let provisioners = self.get_provisioners().await?;
    let data = provisioners
        .iter()
        .map(|(pk, stake)| (pk.inner().clone(), stake.clone()))
        .collect();
    Ok(data)
}
```

### Explanation

Adds full provisioner enumeration and per-public key stake lookups not supported by the HTTP server.

---

## 6. query_contract_raw

### Legacy HTTP

GraphQL and streaming RUES events handle contract queries via schema; no direct raw contract call.

### JSON-RPC VM Adapter

```rust
async fn query_contract_raw(
    &self,
    contract_id: dusk_core::abi::ContractId,
    method: String,
    base_commit: [u8; 32],
    args_bytes: Vec<u8>,
) -> Result<Vec<u8>, VmError> {
    let node = Arc::clone(&self.node_rusk);
    let result = tokio::task::spawn_blocking(move || {
        // Open a session at `base_commit`
        let mut session = node
            .query_session(Some(base_commit))
            .map_err(|e| VmError::QueryFailed(e.to_string()))?;
        // Execute raw call
        let receipt = session
            .call_raw(
                contract_id,
                method.as_ref(),
                args_bytes,
                node.vm_config.block_gas_limit,
            )
            .map_err(|e| VmError::QueryFailed(e.to_string()))?;
        Ok(receipt.data)
    })
    .await
    .map_err(|e| VmError::InternalError(e.to_string()))?;
    result
}
```

### Explanation

Exposes raw contract calls for arbitrary methods and arguments, with explicit control over the base state commit—absent in the legacy HTTP interface.

---

## 7. get_vm_config

### Legacy HTTP (`node/info` endpoint)

```rust
// inside `get_info()` handler
let vm_conf = self.inner().vm_handler().read().await.vm_config.clone();
info.insert("vm_config", serde_json::to_value(vm_conf)?);
```

### JSON-RPC VM Adapter (`rusk/src/lib/jsonrpc/infrastructure/vm.rs`)

```rust
async fn get_vm_config(&self) -> Result<RuskVmConfig, VmError> {
    Ok(self.node_rusk.vm_config.clone())
}
```

### Explanation

Both surfaces the node's VM configuration; the adapter returns it directly over JSON-RPC, while HTTP embeds it in the `/node/info` payload.

---
