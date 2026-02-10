// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  AccountSyncer,
  Bookkeeper,
  Network,
  ProfileGenerator,
} from "@dusk/w3sper";

import { assert, NETWORK, seeder, test, Treasury } from "./harness.js";

/**
 * Tests fetching the stake information using string representations
 * of accounts, verifying access without requiring instances of
 * ProfileGenerator, Treasury, or Bookkeeper.
 */
test("stake info without profiles", async () => {
  const users = [
    "oCqYsUMRqpRn2kSabH52Gt6FQCwH5JXj5MtRdYVtjMSJ73AFvdbPf98p3gz98fQwNy9ZBiDem6m9BivzURKFSKLYWP3N9JahSPZs9PnZ996P18rTGAjQTNFsxtbrKx79yWu",
    "ocXXBAafr7xFqQTpC1vfdSYdHMXerbPCED2apyUVpLjkuycsizDxwA6b9D7UW91kG58PFKqm9U9NmY9VSwufUFL5rVRSnFSYxbiKK658TF6XjHsHGBzavFJcxAzjjBRM4eF",
  ];

  const network = await Network.connect(NETWORK);
  const syncer = new AccountSyncer(network);

  const stakes = await syncer.stakes(users);

  assert.equal(stakes.length, 2);

  assert.equal(stakes[0].amount.value, 2_000_000_000_000n);
  assert.equal(
    stakes[0].amount.total,
    stakes[0].amount.value + stakes[0].amount.locked,
  );
  assert.equal(stakes[0].amount.locked, 0n);

  // No check for reward's value since it is not deterministic
  assert.equal(typeof stakes[0].reward, "bigint");
  assert.equal(stakes[0].faults, 0);
  assert.equal(stakes[0].hardFaults, 0);

  // No stakes for the 2nd user
  assert.equal(stakes[1].amount, null);
  assert.equal(stakes[1].reward, 0n);
  assert.equal(stakes[1].faults, 0);
  assert.equal(stakes[1].hardFaults, 0);

  await network.disconnect();
});

/**
 * Test fetching the stake information using profiles and treasury/bookkeeper
 * instances.
 *
 * Although this requires more code than using the syncer directly, it enables
 * the use of a decoupled cache to retrieve and store the stake information.
 */
test("stake info with treasury", async () => {
  const network = await Network.connect(NETWORK);

  const profiles = new ProfileGenerator(seeder);
  const users = await Promise.all([profiles.default, profiles.next()]);

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookkeeper = new Bookkeeper(treasury);

  const stakes = await Promise.all([
    bookkeeper.stakeInfo(users[0].account),
    bookkeeper.stakeInfo(users[1].account),
    bookkeeper.as(users[0]).info.stake(),
  ]);

  assert.equal(stakes.length, 3);

  // Stake information for the default profile matches
  assert.equal(stakes[0], stakes[2]);

  assert.equal(stakes[0].amount.value, 2_000_000_000_000n);
  assert.equal(
    stakes[0].amount.total,
    stakes[0].amount.value + stakes[0].amount.locked,
  );
  assert.equal(stakes[0].amount.locked, 0n);

  // No check for reward's value since it is not deterministic
  assert.equal(typeof stakes[0].reward, "bigint");
  assert.equal(stakes[0].faults, 0);
  assert.equal(stakes[0].hardFaults, 0);

  // No stakes for the 2nd user
  assert.equal(stakes[1].amount, null);
  assert.equal(stakes[1].reward, 0n);
  assert.equal(stakes[1].faults, 0);
  assert.equal(stakes[1].hardFaults, 0);

  await network.disconnect();
});
