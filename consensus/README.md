 # Consensus protocol
  Consensus protocol called Segregated Byzantine Agreement (SBA from hereon). SBA is a permission-less Proof-of-Stake-based consensus mechanism with statistical finality guarantees. SBA belongs to the committee-based PoS subcategory due to its use of committees to finalize a block for a given round.
 
   SBA is permission-less, meaning that any eligible (eligibility properties are defined below) Dusk Network protocol participant is authorized to join and participate in the consensus. An eligible participant is defined with the following properties:
 
- A participant with a pre-configured amount of DUSK locked as a stake (a.k.a Provisioner)
- A participant, whose stake has a maturity of at least 2 epochs (a.k.a Eligible Provisioner)

Other terms used below:
- `Round` - A single SBA execution
- `Round iteration` - All three phases (Selection, 1th Reduction and 2th Reduction) executed in a row.
- `Committee` - A subset of Eligible Provisioners, extracted on performing `Deterministic Sortition`

# Repository structure

## Example Node
A minimalistic and stateless version of dusk-blockchain node where Consensus protocol in conjuction with Kadcast is able to join and participate `dusk-blockchain`/test-harness. Useful for testing and diagnostic incompatibility issues. This executable should be deprecated once dusk-blockchain is fully migrated.

## Example Testbed
A multi-instance setup running 10 SBA instances provisioned with 10 eligible participants. The setup is configured to run for up to 1000 rounds. Useful for any kind of testing (issues, stress and performance testing).

## Consensus crate
A full implementation of SBA mechanism.

 # Implementation details 
SBA is driven basically by two tokio-rs tasks - Main Loop and Agreement Loop. Each of them has its own inbound and outbound message queues/channels to exchange messages with outsiders. SBA protocol parameters are defined in `/src/config.rs`.

- `Main_Loop uses` `Operations trait` to execute Contract Storage calls and `Database trait` to store and retrieve candidate blocks. It is mainly responsible to execute Selection, 1th Reduction and 2nd Reduction steps in a row and eventually produce/broadcast an **Agreement Message**. Inbound queue for Main_Loop can contain messages of either **NewBlock** or **Reduction** type.


- `Agreement_Loop` uses `Database trait` to retrieve candidate blocks when a winner hash is found. It is mainly responsible to verify and accumulate **Agreement messages** from different Consensus iterations and process **Aggregated Agreement message**. Inbound queue for Agreement_Loop can contain messages of either **Agreement** or **AggrAgreement** type. An Agreement message is verified and accumulated concurrently by a pool of workers/verifiers - again tokio-rs tasks.


 ![Screenshot](node.png)


## How to use (example code)
```rust
let mut consensus = Consensus::new(
	inbound_msgs,
	outbound_msgs,
	agr_inbound_queue,
	agr_outbound_queue,
	Arc::new(Mutex::new(crate::mocks::Executor {})),
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
	/// and spawns all consensus tokio::tasks.
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
export DUSK_WALLET_DIR="TODO"
export DUSK_CONSENSUS_KEYS_PASS="TODO"

USAGE:
    node --bootstrap <bootstrap>... --address <public_address>  --preloaded-num <preloaded-num> --provisioner-unique-id <prov-id>  --log-level <LOG>

```


