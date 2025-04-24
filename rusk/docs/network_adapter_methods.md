# Network Adapter vs. Legacy HTTP Server – Method Comparison

This document compares each `NetworkAdapter` method in
`rusk/src/lib/jsonrpc/infrastructure/network.rs` against
the approach used in the legacy HTTP server (`rusk/src/lib/http/chain.rs`),
with code snippets and explanations of any differences.

---

## 1. broadcast_transaction

### Legacy HTTP Implementation (`rusk/src/lib/http/chain.rs`)

```rust
async fn propagate_tx(&self, tx: &[u8]) -> anyhow::Result<ResponseData> {
    let tx: Transaction = ProtocolTransaction::from_slice(tx)
        .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?
        .into();
    let network = self.network();
    // Internally routes the message via Kadcast
    network.read().await.route_internal(tx);
    Ok(ResponseData::new(DataType::None))
}
```

### JSON-RPC Network Adapter (`rusk/src/lib/jsonrpc/infrastructure/network.rs`)

```rust
async fn broadcast_transaction(
    &self,
    tx_bytes: Vec<u8>,
) -> Result<(), NetworkError> {
    // Deserialize ledger::Transaction
    let tx = ledger::Transaction::read(&mut tx_bytes.as_slice())
        .map_err(|e| NetworkError::QueryFailed(e.to_string()))?;
    // Wrap in Message
    let msg = Message::from(tx);
    let client = self.network_client.read().await;
    // Direct broadcast via node::Network::broadcast
    client
        .broadcast(&msg)
        .await
        .map_err(|e| NetworkError::QueryFailed(e.to_string()))
}
```

### Explanation

- **Routing vs. Broadcasting**: The HTTP handler calls `route_internal` to inject into the node's internal pipeline, while the Adapter uses the public `broadcast` API to fire-and-forget across the network.
- **Error Mapping**: Both convert deserialization or network errors into an RPC-friendly error type.

---

## 2. get_network_info

### Legacy HTTP Implementation (`rusk/src/lib/http/chain.rs`)

```rust
async fn get_info(&self) -> anyhow::Result<ResponseData> {
    // Gather network config from RuskNode
    let n_conf = self.network().read().await.conf().clone();
    let mut info = serde_json::Map::new();
    info.insert("bootstrapping_nodes", n_conf.bootstrapping_nodes.into());
    info.insert("chain_id", n_conf.kadcast_id.into());
    info.insert("kadcast_address", n_conf.public_address.into());
    // ... additional fields ...
    Ok(ResponseData::new(info))
}
```

### JSON-RPC Network Adapter (`rusk/src/lib/jsonrpc/infrastructure/network.rs`)

```rust
async fn get_network_info(&self) -> Result<String, NetworkError> {
    let client = self.network_client.read().await;
    client
        .get_info()
        .map_err(|e| NetworkError::QueryFailed(e.to_string()))
}
```

### Explanation

- **Structured vs. Raw String**: The HTTP server builds a JSON map of multiple fields, while the Adapter returns the raw string that `node::Network::get_info()` provides (often a space- or comma-separated summary).
- **Endpoint Location**: HTTP's `/node/info` combines node and network details; JSON-RPC splits network info into its own RPC method.

---

## 3. get_public_address

### Legacy HTTP Implementation

(Inside the same `get_info` handler)

```rust
info.insert(
    "kadcast_address",
    n_conf.public_address.into(),
);
```

### JSON-RPC Network Adapter

```rust
async fn get_public_address(&self) -> Result<SocketAddr, NetworkError> {
    let client = self.network_client.read().await;
    // node::Network::public_addr returns &SocketAddr
    Ok(*client.public_addr())
}
```

### Explanation

Both expose the node's public Kadcast address; the Adapter does so via a dedicated RPC method, while HTTP embeds it in the `/node/info` response.

---

## 4. get_alive_peers

### Legacy HTTP Implementation (`rusk/src/lib/http/chain.rs`)

```rust
// RUES endpoint "network/peers"
async fn handle_rues(&self, request: &RuesDispatchEvent) -> anyhow::Result<ResponseData> {
    let amount = request.data.as_string().trim().parse()?;
    // Calls RuskNode::alive_nodes()
    self.alive_nodes(amount).await
}

async fn alive_nodes(&self, amount: usize) -> anyhow::Result<ResponseData> {
    let peers = self.network().read().await.alive_nodes(amount).await;
    let addrs: Vec<String> = peers.iter().map(|a| a.to_string()).collect();
    Ok(ResponseData::new(serde_json::to_value(addrs)?))
}
```

### JSON-RPC Network Adapter

```rust
async fn get_alive_peers(
    &self,
    max_peers: usize,
) -> Result<Vec<SocketAddr>, NetworkError> {
    let client = self.network_client.read().await;
    client.alive_nodes(max_peers).await;
    Ok(peers)
}
```

### Explanation

- Both list up to `N` currently active peers. HTTP then serializes to strings for JSON; the Adapter returns the raw `SocketAddr` vector.

---

## 5. get_alive_peers_count

### Legacy HTTP

No dedicated HTTP endpoint for peer count.

### JSON-RPC Network Adapter

```rust
async fn get_alive_peers_count(&self) -> Result<usize, NetworkError> {
    let client = self.network_client.read().await;
    Ok(client.alive_nodes_count().await)
}
```

### Explanation

Provides peer-count support for JSON-RPC clients; HTTP never exposed this metric directly.

---

## 6. flood_request

### Legacy HTTP

No HTTP endpoint uses inventory flooding (`flood_request`).

### JSON-RPC Network Adapter

```rust
async fn flood_request(
    &self,
    inv: Inv,
    ttl_seconds: Option<u64>,
    hops: u16,
) -> Result<(), NetworkError> {
    let client = self.network_client.read().await;
    client
        .flood_request(&inv, ttl_seconds, hops)
        .await
        .map_err(|e| NetworkError::QueryFailed(e.to_string()))
}
```

### Explanation

Enables JSON-RPC clients to propagate inventory messages (e.g. block or tx inventory) with TTL/hops parameters; not available in the legacy HTTP API.

---
