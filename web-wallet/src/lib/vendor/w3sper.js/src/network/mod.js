// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../protocol-driver/mod.js";

import { Rues } from "../rues/mod.js";
import { Node } from "./components/node.js";
import { Transactions } from "./components/transactions.js";
import { Contracts } from "./components/contracts.js";
import { Gas } from "./gas.js";

export { Gas };
export { AddressSyncer } from "./syncer/address.js";
export { AccountSyncer } from "./syncer/account.js";
export { Bookmark } from "./bookmark.js";

const protocol = { "https:": "wss:", "http:": "ws:" };

const abortable = (signal) =>
  new Promise((resolve, reject) =>
    signal?.aborted ? reject(signal.reason) : resolve(signal)
  );

const once = (target, topic) =>
  new Promise((resolve) =>
    target.addEventListener(topic, resolve, { once: true })
  );

export class Network {
  #rues;
  node;
  contracts;

  constructor(url, options = {}) {
    this.#rues = new Rues(url, options);
    this.node = new Node(this.#rues);
    this.contracts = new Contracts(this.#rues);
    this.transactions = new Transactions(this.#rues);
  }

  get url() {
    return this.#rues.url;
  }

  get rues() {
    return this.#rues;
  }

  static connect(url, options = {}) {
    return new Network(url).connect(options);
  }

  async connect(options = {}) {
    await this.#rues.connect(options);

    ProtocolDriver.load(
      new URL("$lib/vendor/wallet_core.wasm", import.meta.url)
    );

    return this;
  }

  async disconnect() {
    await ProtocolDriver.unload();

    await this.#rues.disconnect();
  }

  get connected() {
    return this.#rues.connected;
  }

  // TODO: GraphQL returns a `Number` while the block height is a `BigInt`.
  // A `Number` loses precision after 9_007_199_254_740_991, while a `BigInt`
  // can go up to: `18_446_744_073_709_551_615`.
  //
  // I suspect is a GraphQL limitation. In the meantime we convert the `Number`
  // to a `BigInt` for consistency and future proof of the API's consumers.
  get blockHeight() {
    return this.query("query { block(height: -1) { header { height } }}").then(
      (body) => BigInt(body?.block?.header?.height ?? 0)
    );
  }

  async execute(builder) {
    const tx = await builder.build(this);

    // Attempt to preverify the transaction
    await this.transactions.preverify(tx.buffer);

    // Attempt to propagate the transaction
    await this.transactions.propagate(tx.buffer);

    return tx;
  }

  async prove(circuits) {
    return this.#rues
      .scope("prover")
      .call.prove(circuits, {
        headers: { "Content-Type": "application/octet-stream" },
      })
      .then((response) => response.arrayBuffer());
  }

  async query(gql, options = {}) {
    const response = await this.#rues.scope("graphql").call.query(gql, options);

    switch (response.status) {
      case 200:
        return await response.json();
      case 500:
        throw new Error((await response.json())[0]);
      default:
        throw new Error(
          `Unexpected [${response.status}] : ${response.statusText}}`
        );
    }
  }
}
