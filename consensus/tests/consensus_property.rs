// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::Cursor;

use node_data::message::Message;
use node_data::Serializable;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
#[ignore = "fuzz-like deserialization; run manually"]
fn fuzz_message_deserialization_does_not_panic() {
    let mut rng = StdRng::seed_from_u64(4242);

    for _ in 0..1000 {
        let len = rng.gen_range(0..512);
        let mut bytes = vec![0u8; len];
        rng.fill(&mut bytes[..]);

        let result = std::panic::catch_unwind(|| {
            let mut cursor = Cursor::new(bytes);
            let _ = Message::read(&mut cursor);
        });

        assert!(result.is_ok(), "deserialization panicked");
    }
}
