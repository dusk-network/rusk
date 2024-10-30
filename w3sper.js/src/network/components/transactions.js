// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class Transactions {
  #scope = null;

  constructor(rues) {
    this.#scope = rues.scope("transactions");
  }

  preverify(tx) {
    return this.#scope.call
      .preverify(tx.valueOf(), {
        headers: { "Content-Type": "application/octet-stream" },
      })
      .then(({ body }) => body.cancel())
      .then(() => tx);
  }

  propagate(tx) {
    return this.#scope.call
      .propagate(tx.valueOf(), {
        headers: { "Content-Type": "application/octet-stream" },
      })
      .then(({ body }) => body.cancel())
      .then(() => tx);
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
