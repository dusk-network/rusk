// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { Gas } from "../../gas.js";
import { RuesScope } from "../../rues/scope.js";
import { RuesEvent } from "../../rues/event.js";
import * as base16 from "../../encoders/b16.js";

class TransactionExecutedEvent extends RuesEvent {
  constructor(type) {
    super(type);
  }

  get gasPaid() {
    return new Gas({
      limit: this.payload["gas_spent"],
      price: this.payload.inner.fee["gas_price"],
    }).total;
  }

  memo(options = {}) {
    const memo = this.payload.inner.memo;
    if ( typeof memo !== "string" || memo.length === 0) {
      return null;
    }
    const buffer = base16.decode(memo);

    if (options.as === "string") {
      return new TextDecoder().decode(buffer);
    }

    return buffer;
  }
}

export class Transactions extends RuesScope {
  #scope = null;

  constructor(rues) {
    super("transactions");
    this.#scope = rues.scope(this);
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

  eventFrom(ruesEvent) {
    switch (ruesEvent.origin.topic) {
      case "executed":
        return TransactionExecutedEvent.from(ruesEvent);
    }

    return ruesEvent;
  }
}
