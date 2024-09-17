// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class NetworkError extends Error {
  constructor(message) {
    const [, name, description] = message.match(
      /([A-Za-z\-]+?[ ]?Error)[: ]? ?(.*)/
    ) ?? [, "NetworkError", message];

    super(description);
    this.name = name;
  }
}
