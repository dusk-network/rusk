// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

let memory;

export function oninit(instance) {
  memory = instance.exports.memory;
}

export default {
  env: {
    /**
     * This method is required to signal to the host any
     * string messages, for example if the WebAssembly module panics.
     *
     * @param cstr_ptr {number}
     */
    sig(cstr_ptr) {
      let messageBuffer = new Uint8Array(memory.buffer, cstr_ptr);

      const nullTerminatorIndex = messageBuffer.indexOf(0);
      const cstring = messageBuffer.subarray(0, nullTerminatorIndex);

      const message = new TextDecoder().decode(cstring);
      console.log("WASM:", message);
    },
  },
};
