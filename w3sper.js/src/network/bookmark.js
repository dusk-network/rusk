// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export const intoBookmark = Symbol("bookmark::into");

export class Bookmark {
  #data = new Uint8Array(8).fill(0xff);

  constructor(data) {
    if (data instanceof Uint8Array && data.byteLength === 8) {
      this.#data = data;
    } else {
      throw new TypeError("Bookmark must be an 8-byte Uint8Array");
    }
  }

  static from(source) {
    if (typeof source?.[intoBookmark] === "function") {
      return source[intoBookmark]();
    } else if (typeof source === "bigint" || typeof source === "number") {
      let buffer = new ArrayBuffer(8);
      new DataView(buffer).setBigUint64(0, BigInt(source), true);
      return new Bookmark(new Uint8Array(buffer));
    } else if (source instanceof Bookmark) {
      return new Bookmark(source.data);
    }

    let buffer = Uint8Array.from(data);
    return new Bookmark(buffer);
  }

  get data() {
    return this.#data;
  }

  asUint() {
    return new DataView(this.#data.buffer).getBigUint64(0, true);
  }

  toString() {
    return this.#data
      .map((byte) => byte.toString(16).padStart(2, "0"))
      .join("");
  }

  // TODO: change name
  isNone() {
    return this.#data.every((byte) => byte === 0xff);
  }
}
