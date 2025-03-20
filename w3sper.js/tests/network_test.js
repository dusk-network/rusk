// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { Network } from "@dusk/w3sper";

import { assert, FakeWebSocket, resolveAfter, test } from "./harness.js";

test("Network connection", async () => {
  const network = new Network("http://localhost:8080/");

  assert.ok(!network.connected);

  assert.equal(Object.keys(await network.node.info), [
    "bootstrappingNodes",
    "chainId",
    "kadcastAddress",
    "version",
    "versionBuild",
    "vmConfig",
    "chain",
  ]);

  const { chain, chainId } = await network.node.info;

  assert.equal(+chain, 0);
  assert.equal(+chain, chainId);
  assert.equal(chain.toString(), "localnet");
  assert.ok(chain === Network.LOCALNET);

  await network.connect();

  assert.ok(network.connected);

  await network.disconnect();
});

test("Network connection failure", async () => {
  assert.throws(() => Network.connect(), TypeError);

  const signal = AbortSignal.timeout(10);

  await resolveAfter(11, undefined);

  const timeoutError = await assert.reject(
    () => Network.connect("http://localhost:8080/", { signal }),
    DOMException
  );

  assert.ok(timeoutError.name === "TimeoutError");

  const networkError = await assert.reject(
    () => Network.connect("http://localhost.fake:8080/"),
    DOMException
  );

  assert.ok(networkError.name === "NetworkError");
});

test("Network's RUES failure after connection is established", async () => {
  const handledEvents = {
    connect: 0,
    disconnect: 0,
    error: 0,
  };
  const countEvent = (event) => {
    handledEvents[event.type]++;
  };
  const RealWebSocket = WebSocket;

  globalThis.WebSocket = FakeWebSocket;

  const network = await Network.connect("http://localhost:8080/", {
    signal: AbortSignal.timeout(100),
  });

  network.addEventListener("connect", countEvent);
  network.addEventListener("disconnect", countEvent);
  network.addEventListener("error", countEvent);

  // this would wait indefinitely if we didn't trigger the timeout
  const resultA = await assert.reject(() =>
    network.transactions.withId("foo-id").once.removed()
  );

  assert.ok(resultA instanceof CustomEvent);
  assert.ok(resultA.type === "disconnect");
  assert.ok(!network.connected);

  await network.connect();

  const fakeError = new Error("some message");

  FakeWebSocket.triggerSocketError(fakeError, 100);

  // this would wait indefinitely if we didn't trigger an error
  const resultB = await assert.reject(() =>
    network.transactions.withId("foo-id").once.removed()
  );

  assert.ok(resultB instanceof ErrorEvent);
  assert.ok(resultB.type === "error");
  assert.ok(resultB.error === fakeError);

  await network.disconnect();

  assert.ok(handledEvents.connect === 1);
  assert.ok(handledEvents.disconnect === 2);
  assert.ok(handledEvents.error === 1);

  globalThis.WebSocket = RealWebSocket;
});

test("Network's disconnect event is fired when the socket closes on its own", async () => {
  const RealWebSocket = WebSocket;

  globalThis.WebSocket = FakeWebSocket;

  let firedDisconnect = false;

  const network = await Network.connect("http://localhost:8080/");

  network.addEventListener("disconnect", () => {
    firedDisconnect = true;
  });

  FakeWebSocket.triggerSocketClose();

  await resolveAfter(100, undefined);

  assert.ok(firedDisconnect);
  assert.ok(!network.connected);

  await network.disconnect();

  globalThis.WebSocket = RealWebSocket;
});

test("Multiple connection calls won't create new sockets if the network is already connected", async () => {
  let connectCalls = 0;
  const network = new Network("http://localhost:8080/");

  network.addEventListener("connect", () => {
    connectCalls++;
  });

  network.connect();
  network.connect();
  await network.connect();

  assert.ok(connectCalls === 1);

  await network.disconnect();
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
