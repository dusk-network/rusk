# Consensus Protocol

The Dusk Network utilizes a consensus protocol called Succinct Attestation (SA). SA is a permissionless proof-of-stake (PoS) consensus mechanism that provides statistical finality guarantees[^1]. SA belongs to the committee-based PoS category[^2] because it uses committees to finalize blocks for a given round.

SA is permissionless, meaning that any eligible participant in the Dusk Network protocol can join and participate in the consensus process. To be eligible, a participant must meet the following requirements:

 - Have a pre-configured amount of DUSK locked as a stake (referred to as a `Provisioner`)
 - Have a stake with a maturity of at least two epochs (referred to as an `Eligible Provisioner`)

Other terms used in the context of SA:

 - `Round`: A single SA execution.
 - `Round iteration`: The execution of all three phases (Selection, 1st Reduction, and 2nd Reduction) in a row.
 - `Committee`: A subset of Eligible Provisioners, selected through a process called "Deterministic Sortition."

# Repository structure

## Example Node
The minimalistic and stateless version of the dusk-blockchain node allows for testing and diagnosing compatibility issues using the Consensus protocol in conjunction with Kadcast[^3]. It enables the node to join and participate in the dusk-blockchain/test-harness. Once the dusk-blockchain is fully migrated, this executable will no longer be needed and can be deprecated.

## Example Testbed
A multi-instance setup running 10 SA instances provisioned with 10 eligible participants. The setup is configured to run for up to 1000 rounds. Useful for any kind of testing (issues, stress and performance testing).

## Consensus crate
A full implementation of SA mechanism.

# Implementation details
The implementation of *SA* consists of two main `tokio-rs` tasks, the `Main_Loop` and `Agreement_Loop`, which communicate with external components through message queues/channels. The protocol parameters for *SA* are located in `src/config.rs`.

- The `Main_Loop` is responsible for executing contract storage calls using the `Operations` trait and storing and retrieving candidate blocks using the `Database` trait. It performs the selection, first reduction, and second reduction steps[^4] in sequence and ultimately produces and broadcasts an `Agreement Message`. The inbound queue for the `Main_Loop` can contain either **NewBlock** or **Reduction** type messages.

- The `Agreement_Loop` retrieves a candidate block using the `Database` trait when a winner hash is selected. It is responsible for verifying and accumulating **Agreement** messages from different consensus iterations and processing the **Aggregated Agreement** message. The inbound queue for the `Agreement_Loop` can contain either **Agreement** or **Aggregated Agreement** type messages. The messages are concurrently verified and accumulated by a pool of worker tasks in `tokio-rs`.

 ![Screenshot](node.png)

## How to use (example code)
```rust
let mut consensus = Consensus::new(
	// Inbound messages for Main Loop
	inbound_msgs,
	// Outbound messages for Main Loop
	outbound_msgs,
	// Inbound messages for Agreement Loop
	agr_inbound_queue,
	// Outbound messages for Agreement Loop
	agr_outbound_queue,
	// Implements Operations trait
	Arc::new(Mutex::new(crate::mocks::Executor {})),
	// Implements Database trait
	Arc::new(Mutex::new(crate::mocks::SimpleDB::default())),
);

let mut most_recent_block = Block::default();

loop {
	/// Provisioners list is retrieved from contract storage state.
	let provisioners = rusk::get_provisioners();

	// Round update is the input data for any consensus round execution.
	// Round update includes mostly data from most recent block. 
	let round_update = from(most_recent_block);

	/// Consensus::Spin call initializes a consensus round
	/// and spawns main consensus tokio::tasks.
	let ret = consensus.spin(
			round_update
			provisioners,
			cancel_rx,
		)
		.await;

	/// Consensus spin output/ret can be a winner block or an error. 
	match ret {
		Ok(winner_block) => { 
			println!("new block produced");
		}
		Err(_) => {
			// Cancelled from outside by cancel_rx chan.
			// Max Step Reached - happens only if no consensus is reached for up to 213 steps/71 iterations.
		}
	}
	most_recent_block = winner;

	/// Internally, consensus instance may accept future messages for next round. 
	/// They will be drained on running the round, that's why same consensus instance is used for all round executions.
}
```
 
 ### Tokio runtime

The implementation is fully based on Tokio-rs/Runtime. That said the recommended way of setting up the runtime is shown below.

 ```rust
  tokio::runtime::Builder::new_multi_thread()
			// A thread per an accumulator worker so that CPU-bound operations 
			// (the agreement verification) does not impact negatively main tokio tasks. 
            .worker_threads(2 + consensus::config::ACCUMULATOR_WORKERS_AMOUNT)
			// Enable the time driver so we can use timeout feature in all steps execution.
            .enable_time()
            .build()
            .unwrap()
            .block_on(async { ... } )
 ```

# Build, Run and Test
```bash
# Run unit tests
cargo test
```

```bash
# Build consensus
cargo b --release
```

```bash
# Build and Run in-process testbed example
cargo run --release --example testbed
```

```bash
# Build example node
cargo b --release --example node

# Run example node
export DUSK_WALLET_DIR="TBD"
export DUSK_CONSENSUS_KEYS_PASS="TBD"

USAGE:
    node --bootstrap <bootstrap>... --address <public_address>  --preloaded-num <preloaded-num> --provisioner-unique-id <prov-id>  --log-level <LOG>

```

[^1]: A finality guarantee that is achieved through the accumulation of blocks over time, such that the probability of a block being reversed decreases exponentially as more blocks are added on top of it. This type of guarantee is in contrast to absolute finality, which is achieved when it is mathematically impossible for a block to be reversed.
[^2]: A type of Proof-of-Stake consensus mechanism that relies on a committee of validators, rather than all validators in the network, to reach consensus on the next block. Committee-based PoS mechanisms often have faster block times and lower overhead than their non-committee counterparts, but may also be more susceptible to censorship or centralization.
[^3]: Kadcast is a decentralized protocol used for efficient communication between nodes in a network. It is based on the Kademlia algorithm and maintains a routing table to find the best path to another node. Kadcast is commonly used in decentralized systems, such as decentralized applications (DApps) and decentralized file sharing systems, to send messages and data between nodes in a fast and reliable manner. One of the main advantages of Kadcast is its decentralized nature, which makes it resistant to censorship and other forms of interference, making it a popular choice for applications that require decentralization. Please find the whitepaper [here](https://eprint.iacr.org/2021/996). Dusk's implementation can be found [on its repo](https://github.com/dusk-network/kadcast).
[^4]: In the Selection phase, a replica (or Provisioner in Dusk terminolody) acting as a Block Generator, proposes a block. In the First Reduction phase, a sampled committee of Provisioners vote on the validity of the block by signing a message with a BLS signature and broadcasting it. In the Second Reduction phase, a different sampled committee of Provisioners vote on the block, gather the results of the First Reduction phase, and produce an "Agreement" message. The steps keep iterating until the process is interrupted by the `Agreement` loop. This allows the group to reach a final decision about which block to agree on.
