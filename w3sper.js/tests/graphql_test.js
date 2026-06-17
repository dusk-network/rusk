// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { Network } from "@dusk/w3sper";
import {
  normalizeGraphQLRequest,
  parseGraphQLResponse,
} from "../src/network/graphql.js";
import { assert, DEFAULT_NETWORK, getNetwork, test } from "./harness.js";

test("normalizeGraphQLRequest trims before wrapping field selections", () => {
  assert.equal(
    normalizeGraphQLRequest("  block(height: -1) { header { height } }  ")
      .query,
    "query { block(height: -1) { header { height } } }",
  );
});

test("parseGraphQLResponse reports missing data on empty 200 responses", async () => {
  const response = new Response("", {
    status: 200,
    statusText: "OK",
    headers: { "Content-Type": "application/json" },
  });

  const error = await assert.reject(
    () => parseGraphQLResponse(response),
    Error,
  );

  assert.equal(
    error.message,
    "Invalid GraphQL response: missing data/errors",
  );
});

test("Network.query posts GraphQL JSON to /graphql", async () => {
  const originalFetch = globalThis.fetch;
  const network = new Network(DEFAULT_NETWORK);
  let received;

  globalThis.fetch = (resource, options) => {
    received = { resource, options };

    return Response.json({ data: { ping: "pong" } });
  };

  try {
    const result = await network.query("ping");
    const { body, headers, method } = received.options;

    assert.equal(received.resource.toString(), `${DEFAULT_NETWORK}graphql`);
    assert.equal(method, "POST");
    assert.equal(
      headers.get("accept"),
      "application/graphql-response+json, application/json",
    );
    assert.equal(headers.get("content-type"), "application/json");
    assert.equal(headers.get("rusk-version"), network.rues.version);
    assert.equal(body, JSON.stringify({ query: "query { ping }" }));
    assert.equal(result.ping, "pong");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("getNetwork falls back when Deno env access is unavailable", () => {
  const originalGet = Deno.env.get;

  Deno.env.get = () => {
    throw new Deno.errors.PermissionDenied("env access denied");
  };

  try {
    assert.equal(getNetwork(), DEFAULT_NETWORK);
  } finally {
    Deno.env.get = originalGet;
  }
});
