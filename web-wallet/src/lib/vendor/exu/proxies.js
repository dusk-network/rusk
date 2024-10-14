// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export const NullTarget = Object.create(null);

export const MemoryProxy = new Proxy(NullTarget, {
  get(...args) {
    throw new ReferenceError(
      "Cannot directly access to WebAssembly Memory unless it's a Shared Memory"
    );
  },
});
