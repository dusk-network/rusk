// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const snakeToCamel = (name) =>
  name.replace(/_([a-z])/g, (_, ch) => ch.toUpperCase());

export class Node {
  #scope = null;
  #info = null;

  constructor(rues) {
    this.#scope = rues.scope("node");
  }

  get info() {
    if (this.#info) {
      return this.#info;
    }

    return this.#scope.call
      .info()
      .then((r) => r.json())
      .then((data) =>
        Object.fromEntries(
          Object.entries(data).map(([key, value]) => [
            snakeToCamel(key),
            value,
          ]),
        ),
      )
      .then((info) => {
        info.chainId = info.chainId ?? 0;
        this.#info = Object.freeze(info);
        return this.#info;
      });
  }

  crs() {
    return this.#scope.call
      .crs(null, {
        headers: {
          Accept: "application/octet-stream",
        },
      })
      .then((r) => r.arrayBuffer());
  }

  provisioners() {
    return this.#scope.call.provisioners().then((r) => r.json());
  }
}
