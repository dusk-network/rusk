// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class Blocks {
  #scope = null;

  constructor(rues) {
    this.#scope = rues.scope("blocks");
  }

  get gasPrice() {
    // The gas price endpoint returns the current gas price as `number` but it should
    // be `BigInt` to avoid precision loss.
    return this.#scope.call["gas-price"]()
      .then((r) => r.json())
      .then((data) =>
        Object.fromEntries(
          Object.entries(data).map(([key, value]) => [key, BigInt(value)]),
        )
      );
  }

  get on() {
    return this.#scope.on;
  }

  get once() {
    return this.#scope.once;
  }

  get call() {
    return this.#scope.call;
  }

  withId(id) {
    return this.#scope.withId(id);
  }
}
