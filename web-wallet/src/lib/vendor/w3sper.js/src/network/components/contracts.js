// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class Contracts {
  #scope = null;

  constructor(rues) {
    this.#scope = rues.scope("contracts", {
      headers: { "Content-Type": "application/octet-stream" },
    });
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

  get transferContract() {
    return this.withId(
      "0100000000000000000000000000000000000000000000000000000000000000"
    );
  }

  get stakeContract() {
    return this.withId(
      "0200000000000000000000000000000000000000000000000000000000000000"
    );
  }
}
