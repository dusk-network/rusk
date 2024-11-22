// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { Network } from "@dusk/w3sper";

import { test, assert } from "./harness.js";

test("Network connection", async () => {
  const network = new Network("http://localhost:8080/");

  assert.ok(!network.connected);

  assert.equal(Object.keys(await network.node.info), [
    "bootstrappingNodes",
    "chainId",
    "kadcastAddress",
    "version",
    "versionBuild",
    "chain",
  ]);

  const { chain, chainId } = await network.node.info;

  assert.equal(+chain, 0);
  assert.equal(+chain, chainId);
  assert.equal(chain.toString(), "localnet");
  assert.ok(chain === Network.LOCALNET);
});

test("Network block height", async () => {
  const network = await Network.connect("http://localhost:8080/");

  assert.ok((await network.blockHeight) > 0);

  await network.disconnect();
});

test("Network gas price", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const price = await network.blocks.gasPrice;

  assert.equal(typeof price.average, "bigint");
  assert.equal(typeof price.max, "bigint");
  assert.equal(typeof price.median, "bigint");
  assert.equal(typeof price.min, "bigint");

  await network.disconnect();
});
