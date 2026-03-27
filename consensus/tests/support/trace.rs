// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::PathBuf;

use node_data::message::{Message, Payload};

use super::{write_trace_with_meta, TraceEntry, TraceMeta};

pub fn track_trace_round_prev(
    msg: &Message,
    expected_round: u64,
    expected_prev: [u8; 32],
    round_prev: &mut HashMap<u64, [u8; 32]>,
) {
    match msg.payload {
        Payload::Candidate(_)
        | Payload::Validation(_)
        | Payload::Ratification(_)
        | Payload::ValidationQuorum(_)
        | Payload::Quorum(_) => {
            let round = msg.header.round;
            let prev = msg.header.prev_block_hash;
            if let Some(existing) = round_prev.get(&round) {
                assert_eq!(
                    existing, &prev,
                    "inconsistent prev_block_hash in trace for round {round}"
                );
            } else {
                if round == expected_round {
                    assert_eq!(
                        prev, expected_prev,
                        "unexpected prev_block_hash in trace message"
                    );
                }
                round_prev.insert(round, prev);
            }
        }
        _ => {}
    }
}

pub fn write_failure_trace(
    network_seed: u64,
    nodes: usize,
    trace: &[TraceEntry],
) -> PathBuf {
    let path = if let Ok(path) = std::env::var("DUSK_CONSENSUS_TRACE_FAIL_OUT")
    {
        PathBuf::from(path)
    } else {
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_millis();
        path.push(format!(
            "dusk-consensus-fault-trace-{network_seed}-{stamp}.log"
        ));
        path
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let meta = TraceMeta {
        seed: Some(network_seed),
        nodes: Some(nodes),
    };
    write_trace_with_meta(&path, trace, &meta);
    path
}
