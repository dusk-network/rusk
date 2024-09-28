// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../src/protocol-driver/mod.js";
import { ProfileGenerator } from "./profile.js";

export class Bookkeeper {
  #treasury;
  constructor(treasury) {
    this.#treasury = treasury;
  }

  async balance(profileType) {
    const seed = await ProfileGenerator.seedFrom(profileType);
    const type = ProfileGenerator.typeOf(profileType.toString());
    const index = +profileType;

    switch (type) {
      case "account":
        return this.#treasury.account(profileType);
        break;
      case "address":
        return ProtocolDriver.balance(
          seed,
          index,
          this.#treasury.address(profileType),
        );
        break;
    }
  }

  async transfer(amount, { from, to }) {
    console.log(`transfer : ${amount}\nfrom     : ${from}\nto       : ${to}`);
  }
}
