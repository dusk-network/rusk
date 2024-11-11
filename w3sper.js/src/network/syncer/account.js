// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../../protocol-driver/mod.js";
import * as base58 from "../../encoders/b58.js";

function intoAccount(resource) {
  if (resource?.account?.valueOf()?.byteLength === 96) {
    return resource.account;
  } else if (typeof resource === "string") {
    const buffer = base58.decode(resource);
    if (buffer.byteLength === 96) {
      return buffer;
    }
  }

  return resource;
}

export class AccountSyncer extends EventTarget {
  #network;

  constructor(network) {
    super();
    this.#network = network;
  }

  async balances(profiles) {
    const rawUsers = await ProtocolDriver.accountsIntoRaw(
      profiles.map(intoAccount),
    );

    let balances = rawUsers.map((user) =>
      this.#network.contracts.transferContract.call.account(user),
    );

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
