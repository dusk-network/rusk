// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// Return a promised rejected if the signal is aborted, resolved otherwise

import * as ProtocolDriver from "../protocol-driver/mod.js";

import { Rues } from "./rues.js";
import { GraphQLRequest } from "./graphql.js";
import { NetworkError } from "./error.js";
import { Gas } from "./gas.js";

export { Gas };
export { AddressSyncer } from "./syncer/address.js";
export { AccountSyncer } from "./syncer/account.js";
export { Bookmark } from "./bookmark.js";

const protocol = { "https:": "wss:", "http:": "ws:" };

const abortable = (signal) =>
  new Promise((resolve, reject) =>
    signal?.aborted ? reject(signal.reason) : resolve(signal),
  );

const once = (target, topic) =>
  new Promise((resolve) =>
    target.addEventListener(topic, resolve, { once: true }),
  );

const snakeToCamel = (name) =>
  name.replace(/_([a-z])/g, (_, ch) => ch.toUpperCase());

export class Network {
  #rues;
  #nodeInfo = null;

  constructor(url) {}

  static connect(url, options = {}) {
    return new Network(url).connect(options);
  }

  async connect(options = {}) {
    const url = new URL(this.url);
    url.protocol = protocol[url.protocol];
    url.pathname = "/on";

    const { signal } = options;
    const socket = new WebSocket(url);
    socket.binaryType = "arraybuffer";
    this.#socket = socket;
    socket.onerror = console.error;
    return new Promise(async (resolve, reject) => {
      if (signal?.aborted) {
        reject(signal.reason);
      } else if (signal) {
        signal.addEventListener("abort", (event) => {
          socket.close();
        });
      }

      await once(socket, "open");
      const event = await once(socket, "message");

      this.#sessionId = event.data;

      socket.onmessage = onMessage;

      const url = new URL("/on/node/info", this.url);

      const response = await this.dispatch(url);

      const info = await response.json();

      const nodeInfo = Object.fromEntries(
        Object.entries(info).map(([key, value]) => [snakeToCamel(key), value]),
      );

      nodeInfo.chainId = nodeInfo.chainId ?? 0;
      this.#nodeInfo = Object.freeze(nodeInfo);

      ProtocolDriver.load(
        new URL("/static/drivers/wallet-core.wasm", this.url),
      );

      resolve(this);
    });
  }

  async disconnect() {
    await ProtocolDriver.unload();

    if (this.connected) {
      this.#socket.close();
      await once(this.#socket, "close");
    }

    this.#nodeInfo = null;
    this.#sessionId = undefined;
  }

  get nodeInfo() {
    return this.#nodeInfo;
  }

  get connected() {
    return this.#socket?.readyState === WebSocket.OPEN;
  }

  get sessionId() {
    return this.#sessionId;
  }

  // TODO: GraphQL returns a `Number` while the block height is a `BigInt`.
  // A `Number` loses precision after 9_007_199_254_740_991, while a `BigInt`
  // can go up to: `18_446_744_073_709_551_615`.
  //
  // I suspect is a GraphQL limitation. In the meantime we convert the `Number`
  // to a `BigInt` for consistency and future proof of the API's consumers.
  get blockHeight() {
    return this.query("query { block(height: -1) { header { height } }}").then(
      (body) => BigInt(body?.block?.header?.height ?? 0),
    );
  }

  async execute(builder) {
    const tx = await builder.build(this);

    // Attempt to preverify the transaction
    await this.preverify(tx.buffer);

    // Attempt to propagate the transaction
    await this.propagate(tx.buffer);

    return tx;
  }

  async prove(circuits) {
    const url = new URL("/on/prover/prove", this.url);

    const req = new Request(url, {
      headers: { "Content-Type": "application/octet-stream" },
      method: "POST",
      body: circuits,
    });

    return this.dispatch(req).then((response) => response.arrayBuffer());
  }

  async preverify(tx) {
    const url = new URL("/on/transactions/preverify", this.url);

    const req = new Request(url, {
      headers: { "Content-Type": "application/octet-stream" },
      method: "POST",
      body: tx.valueOf(),
    });

    const _response = await this.dispatch(req);

    return tx;
  }

  async propagate(tx) {
    const url = new URL("/on/transactions/propagate", this.url);

    const req = new Request(url, {
      headers: { "Content-Type": "application/octet-stream" },
      method: "POST",
      body: tx.valueOf(),
    });

    const response = await this.dispatch(req);
  }

  async dispatch(resource, options = {}) {
    const { signal } = options;

    const headers = new Headers(resource.headers);
    headers.append("rusk-version", "0.8.0");
    headers.append("rusk-session-id", this.#sessionId);

    const response = await fetch(resource, {
      method: "POST",
      headers,
      signal,
    });

    // TODO: In case of mismatching rusk versions, the node *should* return a
    // 4xx status code, however currently it always return a 500 no matter what.
    // We can't rely on the status code to determine the error, so we have to
    // check the response body.
    // This should be fixed on node side.
    if (!response.ok) {
      // We only want to check if this is a version mismatch, but since we
      // have to *consume* the body stream in order to check it, we have to
      // clone the response in case it's not a version mismatch.'

      const resp = response.clone();
      const body = await resp.text();

      if (body.startsWith("Mismatched rusk version:")) {
        throw new Error(body);
      } else {
        throw new NetworkError(body);
      }
    }

    return "handle" in resource ? resource.handle(response) : response;
  }

  query(gql, options = {}) {
    return this.dispatch(new GraphQLRequest(gql, this.url), options);
  }
}
