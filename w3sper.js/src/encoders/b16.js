// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export const encode = (buffer) =>
  Array.from(buffer)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");

export function decode(string) {
  // Check if the string has an even length and contains only valid hex characters
  if (string.length % 2 !== 0 || !/^[\da-fA-F]+$/.test(string)) {
    return null;
  }

  const buffer = new Uint8Array(string.length / 2);

  for (let i = 0; i < string.length; i += 2) {
    buffer[i / 2] = parseInt(string.slice(i, i + 2), 16);
  }

  return buffer;
}
