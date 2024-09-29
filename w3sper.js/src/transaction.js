// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export const TRANSFER =
  "0100000000000000000000000000000000000000000000000000000000000000";

import { AddressSyncer } from "./network/syncer/address.js";
import { Gas } from "./network/gas.js";
import * as ProtocolDriver from "./protocol-driver/mod.js";

export class TransactionBuilder {
  #bookkeeper;

  #from;
  #to;
  #amount;
  #obfuscated = false;
  #gas;

  constructor(bookkeeper) {
    this.#bookkeeper = bookkeeper;

    this.#gas = new Gas();
  }

  from(identifier) {
    this.#from = identifier;
    return this;
  }

  to(identifier) {
    this.#to = identifier;
    return this;
  }

  amount(value) {
    this.#amount = value;
    return this;
  }

  obfuscated() {
    this.#obfuscated = true;
    return this;
  }

  gas(value) {
    this.#gas = value;
    return this;
  }

  async build(network) {
    // Pick notes to spend from the treasury
    const picked = await this.#bookkeeper.pick(
      this.#from,
      this.#amount + this.#gas.total,
    );

    const syncer = new AddressSyncer(network);

    // Fetch the openings from the network for the picked notes
    const openings = (await syncer.openings(picked)).map((opening) => {
      const data = new Uint8Array(opening.slice(4));
      data[0] = 1;
      return data;
    });

    // Fetch the root
    const root = await syncer.root();
    const sender = this.#from;
    const receiver = this.#to;
    const inputs = picked.values();
    const { chainId } = network.nodeInfo;

    await ProtocolDriver.phoenix({
      sender,
      receiver,
      inputs,
      openings,
      root,
      transfer_value: this.#amount,
      obfuscated_transaction: this.#obfuscated,
      deposit: 0n,
      gas_limit: this.#gas.limit,
      gas_price: this.#gas.price,
      chainId,
      data: null,
    });
  }
}
