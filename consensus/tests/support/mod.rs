// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dusk_core::signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey};
use node_data::bls::{PublicKey, PublicKeyBytes};
use node_data::ledger::{Block, Hash, Header, Seed, SpentTransaction};
use node_data::message::payload::ValidationResult;
use node_data::message::{AsyncQueue, ConsensusHeader, Message, Payload};
use node_data::Serializable;
use node_data::StepName;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sha3::{Digest, Sha3_256};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

use dusk_consensus::commons::{Database, RoundUpdate, TimeoutSet};
use dusk_consensus::config::{MIN_STEP_TIMEOUT, TIMEOUT_INCREASE};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::errors::{ConsensusError, HeaderError, OperationError, StateTransitionError};
use dusk_consensus::operations::{Operations, StateTransitionData, StateTransitionResult, Voter};
use dusk_consensus::queue::MsgRegistry;
use dusk_consensus::user::provisioners::{Provisioners, DUSK};

#[derive(Clone)]
pub struct TestKeys {
    pub sk: BlsSecretKey,
    pub pk: PublicKey,
}

impl TestKeys {
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let sk = BlsSecretKey::random(&mut rng);
        let pk = PublicKey::new(BlsPublicKey::from(&sk));
        Self { sk, pk }
    }
}

#[derive(Default)]
pub struct FakeDatabase {
    pub candidates: Vec<Block>,
    pub validation_results: Vec<(ConsensusHeader, ValidationResult)>,
    pub last_iter: (Hash, u8),
}

#[async_trait]
impl Database for FakeDatabase {
    async fn store_candidate_block(&mut self, b: Block) {
        self.candidates.push(b);
    }

    async fn store_validation_result(
        &mut self,
        ch: &ConsensusHeader,
        vr: &ValidationResult,
    ) {
        self.validation_results.push((*ch, vr.clone()));
    }

    async fn get_last_iter(&self) -> (Hash, u8) {
        self.last_iter
    }

    async fn store_last_iter(&mut self, data: (Hash, u8)) {
        self.last_iter = data;
    }
}

#[derive(Debug)]
pub struct OpsConfig {
    pub fail_next_validate_header: bool,
    pub fail_next_state_transition: bool,
    pub gas_limit: u64,
    pub state_root: [u8; 32],
    pub event_bloom: [u8; 256],
}

impl Default for OpsConfig {
    fn default() -> Self {
        Self {
            fail_next_validate_header: false,
            fail_next_state_transition: false,
            gas_limit: 0,
            state_root: [0u8; 32],
            event_bloom: [0u8; 256],
        }
    }
}

#[derive(Clone, Default)]
pub struct FakeOperations {
    config: Arc<Mutex<OpsConfig>>,
}

impl FakeOperations {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set_fail_validate_header(&self) {
        let mut cfg = self.config.lock().await;
        cfg.fail_next_validate_header = true;
    }

    pub async fn set_fail_state_transition(&self) {
        let mut cfg = self.config.lock().await;
        cfg.fail_next_state_transition = true;
    }
}

#[async_trait]
impl Operations for FakeOperations {
    async fn validate_block_header(
        &self,
        _candidate_header: &Header,
        _expected_generator: &PublicKeyBytes,
    ) -> Result<Vec<Voter>, HeaderError> {
        let mut cfg = self.config.lock().await;
        if cfg.fail_next_validate_header {
            cfg.fail_next_validate_header = false;
            return Err(HeaderError::Generic("forced failure"));
        }
        Ok(vec![])
    }

    async fn validate_faults(
        &self,
        _block_height: u64,
        _faults: &[node_data::ledger::Fault],
    ) -> Result<(), OperationError> {
        Ok(())
    }

    async fn validate_state_transition(
        &self,
        _prev_state: [u8; 32],
        _blk: &Block,
        _cert_voters: &[Voter],
    ) -> Result<(), OperationError> {
        let mut cfg = self.config.lock().await;
        if cfg.fail_next_state_transition {
            cfg.fail_next_state_transition = false;
            return Err(OperationError::FailedTransitionVerification(
                StateTransitionError::ExecutionError("forced failure".into()),
            ));
        }
        Ok(())
    }

    async fn generate_state_transition(
        &self,
        _transition_data: StateTransitionData,
    ) -> Result<(Vec<SpentTransaction>, StateTransitionResult), OperationError>
    {
        let mut cfg = self.config.lock().await;
        if cfg.fail_next_state_transition {
            cfg.fail_next_state_transition = false;
            return Err(OperationError::FailedTransitionCreation(
                StateTransitionError::ExecutionError("forced failure".into()),
            ));
        }
        Ok((
            vec![],
            StateTransitionResult {
                state_root: cfg.state_root,
                event_bloom: cfg.event_bloom,
            },
        ))
    }

    async fn add_step_elapsed_time(
        &self,
        _round: u64,
        _step_name: StepName,
        _elapsed: Duration,
    ) -> Result<(), OperationError> {
        Ok(())
    }

    async fn get_block_gas_limit(&self) -> u64 {
        let cfg = self.config.lock().await;
        cfg.gas_limit
    }
}

pub struct TestNode {
    pub keys: TestKeys,
    pub inbound: AsyncQueue<Message>,
    pub outbound: AsyncQueue<Message>,
    pub future_msgs: Arc<Mutex<MsgRegistry<Message>>>,
    pub db: Arc<Mutex<FakeDatabase>>,
    pub ops: FakeOperations,
}

impl TestNode {
    pub fn new(keys: TestKeys) -> Self {
        let inbound = AsyncQueue::bounded(1024, "consensus-inbound");
        let outbound = AsyncQueue::bounded(1024, "consensus-outbound");
        let future_msgs = Arc::new(Mutex::new(MsgRegistry::default()));
        let db = Arc::new(Mutex::new(FakeDatabase::default()));
        let ops = FakeOperations::new();

        Self {
            keys,
            inbound,
            outbound,
            future_msgs,
            db,
            ops,
        }
    }

    pub fn round_update(
        &self,
        tip_header: &Header,
        base_timeouts: TimeoutSet,
    ) -> RoundUpdate {
        RoundUpdate::new(
            self.keys.pk.clone(),
            self.keys.sk.clone(),
            tip_header,
            base_timeouts,
            vec![],
        )
    }

    pub fn spawn_consensus(
        &self,
        round_update: RoundUpdate,
        provisioners: Arc<Provisioners>,
    ) -> (oneshot::Sender<i32>, JoinHandle<Result<(), ConsensusError>>) {
        let consensus = Consensus::new(
            self.inbound.clone(),
            self.outbound.clone(),
            self.future_msgs.clone(),
            Arc::new(self.ops.clone()),
            self.db.clone(),
        );
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            consensus.spin(round_update, provisioners, cancel_rx).await
        });
        (cancel_tx, handle)
    }
}

#[derive(Clone)]
pub struct Envelope {
    pub from: usize,
    pub msg: Message,
}

pub struct BufferedRouter {
    handles: Vec<JoinHandle<()>>,
    rx: Mutex<mpsc::UnboundedReceiver<Envelope>>,
}

#[derive(Clone, Debug)]
pub struct TraceEntry {
    pub from: usize,
    pub deliver_at: u32,
    pub payload_hex: String,
}

const TRACE_VERSION: u32 = 1;

#[derive(Clone, Debug, Default)]
pub struct TraceMeta {
    pub seed: Option<u64>,
    pub nodes: Option<usize>,
}

pub fn encode_message(msg: &Message) -> String {
    let mut buf = Vec::new();
    msg.write(&mut buf).expect("message serialize");
    hex::encode(buf)
}

pub fn decode_message(hex_str: &str) -> Message {
    let bytes = hex::decode(hex_str).expect("valid hex");
    let mut cursor = std::io::Cursor::new(bytes);
    Message::read(&mut cursor).expect("message deserialize")
}

pub fn write_trace_with_meta(
    path: &std::path::Path,
    entries: &[TraceEntry],
    meta: &TraceMeta,
) {
    let mut checksum_input = String::new();
    let mut out = String::new();

    let version_line = format!("#trace_version={TRACE_VERSION}");
    checksum_input.push_str(&version_line);
    checksum_input.push('\n');
    out.push_str(&version_line);
    out.push('\n');

    if let Some(nodes) = meta.nodes {
        let line = format!("#nodes={nodes}");
        checksum_input.push_str(&line);
        checksum_input.push('\n');
        out.push_str(&line);
        out.push('\n');
    }
    if let Some(seed) = meta.seed {
        let line = format!("#seed={seed}");
        checksum_input.push_str(&line);
        checksum_input.push('\n');
        out.push_str(&line);
        out.push('\n');
    }

    for entry in entries {
        let line =
            format!("{},{},{}", entry.from, entry.deliver_at, entry.payload_hex);
        checksum_input.push_str(&line);
        checksum_input.push('\n');
        out.push_str(&line);
        out.push('\n');
    }

    let checksum = Sha3_256::digest(checksum_input.as_bytes());
    let checksum_line = format!("#checksum={}", hex::encode(checksum));
    out.push_str(&checksum_line);
    out.push('\n');

    std::fs::write(path, out).expect("write trace");
}

pub fn read_trace_with_meta(path: &std::path::Path) -> (TraceMeta, Vec<TraceEntry>) {
    let contents = std::fs::read_to_string(path).expect("read trace");
    let mut meta = TraceMeta::default();
    let mut version: Option<u32> = None;
    let mut checksum_hex: Option<String> = None;
    let mut checksum_input = String::new();
    let mut entries = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            if let Some(value) = rest.strip_prefix("checksum=") {
                checksum_hex = Some(value.trim().to_string());
                continue;
            }
            if let Some(value) = rest.strip_prefix("trace_version=") {
                version = Some(value.trim().parse().expect("trace version"));
                checksum_input.push_str(line);
                checksum_input.push('\n');
                continue;
            }
            if let Some(value) = rest.strip_prefix("seed=") {
                if let Ok(seed) = value.trim().parse::<u64>() {
                    meta.seed = Some(seed);
                }
                checksum_input.push_str(line);
                checksum_input.push('\n');
                continue;
            }
            if let Some(value) = rest.strip_prefix("nodes=") {
                if let Ok(nodes) = value.trim().parse::<usize>() {
                    meta.nodes = Some(nodes);
                }
                checksum_input.push_str(line);
                checksum_input.push('\n');
                continue;
            }
            continue;
        }
        checksum_input.push_str(line);
        checksum_input.push('\n');
        let mut parts = line.splitn(3, ',');
        let from = parts.next().expect("from").parse().expect("from");
        let deliver_at = parts
            .next()
            .expect("deliver_at")
            .parse()
            .expect("deliver_at");
        let payload_hex = parts.next().expect("payload").to_string();
        entries.push(TraceEntry {
            from,
            deliver_at,
            payload_hex,
        });
    }
    match version {
        Some(TRACE_VERSION) => {}
        Some(other) => panic!("unsupported trace version: {other}"),
        None => panic!("missing trace version"),
    }

    let checksum_hex = checksum_hex.expect("missing checksum");
    let checksum = Sha3_256::digest(checksum_input.as_bytes());
    let computed = hex::encode(checksum);
    assert_eq!(
        checksum_hex, computed,
        "trace checksum mismatch: expected {checksum_hex}, computed {computed}"
    );

    (meta, entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(contents: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!("dusk-consensus-trace-test-{stamp}.log"));
        std::fs::write(&path, contents).expect("write trace");
        path
    }

    #[test]
    fn trace_requires_version_and_checksum() {
        let path = write_tmp("0,0,deadbeef\n");
        let result = std::panic::catch_unwind(|| {
            let _ = read_trace_with_meta(&path);
        });
        assert!(result.is_err(), "missing version/checksum should fail");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn trace_rejects_checksum_mismatch() {
        let contents = "\
#trace_version=1
#nodes=1
0,0,deadbeef
#checksum=0000
";
        let path = write_tmp(contents);
        let result = std::panic::catch_unwind(|| {
            let _ = read_trace_with_meta(&path);
        });
        assert!(result.is_err(), "checksum mismatch should fail");
        let _ = std::fs::remove_file(&path);
    }
}

impl BufferedRouter {
    pub fn start(nodes: &[TestNode]) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut handles = Vec::new();

        for (idx, node) in nodes.iter().enumerate() {
            let outbound = node.outbound.clone();
            let tx = tx.clone();
            let handle = tokio::spawn(async move {
                loop {
                    match outbound.recv().await {
                        Ok(msg) => {
                            let _ = tx.send(Envelope { from: idx, msg });
                        }
                        Err(_) => break,
                    }
                }
            });
            handles.push(handle);
        }

        Self {
            handles,
            rx: Mutex::new(rx),
        }
    }

    pub async fn recv_batch(&self, timeout: Duration) -> Vec<Envelope> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut batch = Vec::new();
        let mut rx = self.rx.lock().await;

        loop {
            let remaining = match deadline.checked_duration_since(tokio::time::Instant::now()) {
                Some(duration) => duration,
                None => break,
            };
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(env)) => batch.push(env),
                Ok(None) => break,
                Err(_) => break,
            }
        }

        batch
    }

    pub fn stop(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}

pub struct TestNetwork {
    pub nodes: Vec<TestNode>,
    pub provisioners: Arc<Provisioners>,
    pub tip_header: Header,
}

impl TestNetwork {
    pub fn new(num_nodes: usize, seed: u64) -> Self {
        let mut provisioners = Provisioners::empty();
        let mut nodes = Vec::with_capacity(num_nodes);

        for i in 0..num_nodes {
            let keys = TestKeys::from_seed(seed + i as u64 + 1);
            provisioners.add_provisioner_with_value(keys.pk.clone(), 1000 * DUSK);
            nodes.push(TestNode::new(keys));
        }

        let mut tip_header = Header::default();
        tip_header.height = 0;
        tip_header.timestamp = 1;
        tip_header.seed = Seed::from([7u8; 48]);

        Self {
            nodes,
            provisioners: Arc::new(provisioners),
            tip_header,
        }
    }

    pub fn base_timeouts() -> TimeoutSet {
        let mut timeouts = HashMap::new();
        timeouts.insert(StepName::Proposal, MIN_STEP_TIMEOUT);
        timeouts.insert(
            StepName::Validation,
            MIN_STEP_TIMEOUT + TIMEOUT_INCREASE,
        );
        timeouts.insert(
            StepName::Ratification,
            MIN_STEP_TIMEOUT + TIMEOUT_INCREASE + TIMEOUT_INCREASE,
        );
        timeouts
    }

    pub fn fast_timeouts() -> TimeoutSet {
        let mut timeouts = HashMap::new();
        timeouts.insert(StepName::Proposal, Duration::from_millis(200));
        timeouts.insert(StepName::Validation, Duration::from_millis(300));
        timeouts.insert(StepName::Ratification, Duration::from_millis(400));
        timeouts
    }
}

pub async fn wait_for_quorum(
    queue: &AsyncQueue<Message>,
    timeout: Duration,
) -> Option<Message> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let now = tokio::time::Instant::now();
        let remaining = match deadline.checked_duration_since(now) {
            Some(duration) => duration,
            None => return None,
        };
        let recv = tokio::time::timeout(remaining, queue.recv()).await;
        match recv {
            Ok(Ok(msg)) => {
                if matches!(msg.payload, node_data::message::Payload::Quorum(_)) {
                    return Some(msg);
                }
            }
            Ok(Err(_)) => return None,
            Err(_) => return None,
        }
    }
}

pub fn deliver_all(nodes: &[TestNode], envelopes: &[Envelope]) {
    for env in envelopes {
        for (idx, node) in nodes.iter().enumerate() {
            if idx != env.from {
                node.inbound.try_send(env.msg.clone());
            }
        }
    }
}

pub fn find_quorum(envelopes: &[Envelope]) -> Option<Message> {
    for env in envelopes {
        if matches!(env.msg.payload, Payload::Quorum(_)) {
            return Some(env.msg.clone());
        }
    }
    None
}
