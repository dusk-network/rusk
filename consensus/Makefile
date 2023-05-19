test-harness:
	echo "Run test-harness cluster with 8 nodes, all provisioners"
	cargo b --release; ./test-harness.sh 8
testbed:
	echo "Run testbed cluster (single-process)"
	cargo run --release --example testbed
release:
	cargo clippy; cargo b --release
