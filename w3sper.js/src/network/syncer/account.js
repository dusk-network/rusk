// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { Bookmark } from "../bookmark.js";
import * as ProtocolDriver from "../../protocol-driver/mod.js";

export const TRANSFER =
  "0100000000000000000000000000000000000000000000000000000000000000";

export class AccountSyncer extends EventTarget {
  #network;

  constructor(network, options = {}) {
    super();
    this.#network = network;
  }

  get #bookmark() {
    const url = new URL(
      `/on/contracts:${TRANSFER}/num_notes`,
      this.#network.url,
    );

    const request = new Request(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/octet-stream",
      },
    });

    return this.#network
      .dispatch(request)
      .then((response) => response.arrayBuffer())
      .then((buffer) => new Bookmark(new Uint8Array(buffer)));
  }

  async balances(profiles, options = {}) {
    const rawUsers = await ProtocolDriver.accountsIntoRaw(profiles);

    let balances = rawUsers.map(async (body) => {
      const url = new URL(
        `/on/contracts:${TRANSFER}/account`,
        this.#network.url,
      );

      const req = new Request(url, {
        headers: { "Content-Type": "application/octet-stream" },
        method: "POST",
        body,
      });

      return this.#network.dispatch(req);
    });

    return await Promise.all(balances)
      .then((responses) => responses.map((resp) => resp.arrayBuffer()))
      .then((buffers) => Promise.all(buffers))
      .then((buffers) =>
        buffers.map((buffer) => ({
          nonce: new DataView(buffer).getBigUint64(0, true),
          value: new DataView(buffer).getBigUint64(8, true),
        })),
      );
  }
}
