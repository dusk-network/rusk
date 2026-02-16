// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::Seed;
use node_data::StepName;

use dusk_consensus::user::committee::Committee;
use dusk_consensus::user::sortition::Config as SortitionConfig;

use super::TestNetwork;

pub fn build_committee(
    network: &TestNetwork,
    iteration: u8,
    step: StepName,
) -> Committee {
    let round = network.tip_header.height + 1;
    build_committee_for_round(
        network,
        network.tip_header.seed,
        round,
        iteration,
        step,
    )
}

pub fn build_committee_for_round(
    network: &TestNetwork,
    seed: Seed,
    round: u64,
    iteration: u8,
    step: StepName,
) -> Committee {
    let mut exclusion = Vec::new();
    if step != StepName::Proposal {
        let cur_generator =
            network.provisioners.get_generator(iteration, seed, round);
        exclusion.push(cur_generator);
        if dusk_consensus::config::exclude_next_generator(iteration) {
            let next_generator =
                network
                    .provisioners
                    .get_generator(iteration + 1, seed, round);
            exclusion.push(next_generator);
        }
    }
    let cfg = SortitionConfig::new(seed, round, iteration, step, exclusion);
    Committee::new(&network.provisioners, &cfg)
}
