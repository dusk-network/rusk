// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../protocol-driver/mod.js";
import { DataDriverRegistry } from "../data-driver/registry.js";

import { Rues } from "../rues/mod.js";
import { Node } from "./components/node.js";
import { Blocks } from "./components/blocks.js";
import { Transactions } from "./components/transactions.js";
import { Contracts } from "./components/contracts.js";
import { Gas } from "../gas.js";

export { Gas };
export { AddressSyncer } from "./syncer/address.js";
export { AccountSyncer } from "./syncer/account.js";
export { Bookmark } from "./bookmark.js";

function makeErrorEventFrom(value) {
  const error = value instanceof Error ? value : new Error(String(value));

  return new ErrorEvent("error", {
    error,
    message: error.message,
  });
}

export class Network extends EventTarget {
  #rues;
  dataDrivers;
  node;
  contracts;
  blocks;
  transactions;

  static LOCALNET = Node.CHAIN.LOCALNET;
  static MAINNET = Node.CHAIN.MAINNET;
  static TESTNET = Node.CHAIN.TESTNET;
  static DEVNET = Node.CHAIN.DEVNET;

  constructor(url, options = {}) {
    super();

    this.#rues = new Rues(url, options);
    this.dataDrivers = new DataDriverRegistry();
    this.node = new Node(this.#rues);
    this.blocks = new Blocks(this.#rues);
    this.contracts = new Contracts(this.#rues);
    this.transactions = new Transactions(this.#rues);

    const dispatcher = (ruesEvent) => {
      this.dispatchEvent(
        ruesEvent instanceof ErrorEvent
          ? makeErrorEventFrom(ruesEvent.error)
          : new CustomEvent(ruesEvent.type)
      );
    };

    this.#rues.addEventListener("connect", dispatcher);
    this.#rues.addEventListener("disconnect", dispatcher);
    this.#rues.addEventListener("error", dispatcher);
  }

  get url() {
    return this.#rues.url;
  }

  get rues() {
    return this.#rues;
  }

  async connect(options = {}) {
    await this.#rues.connect(options);

    ProtocolDriver.load(new URL("/static/drivers/wallet-core-1.3.0.wasm", this.url));

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
    return this.query("block(height: -1) { header { height } }").then((body) =>
      BigInt(body?.block?.header?.height ?? 0)
    );
  }

  async execute(tx) {
    if (typeof tx?.build === "function") {
      tx = await tx.build(this);
    }

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
    gql = gql ? `query { ${gql} }` : "";

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

  static connect(url, options = {}) {
    return new Network(url).connect(options);
  }
}
