// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { none } from "../none.js";

const _mem = Symbol("allocator::mem");
const _alloc = Symbol("allocator::alloc");

class Box {
  #ptr;
  #size = 0;

  [_mem] = none`Boxed values need a memory allocator.`;

  constructor() {}

  [Symbol.toPrimitive](hint) {
    if (hint === "number" || hint === "default") {
      return this.#ptr;
    }
    return null;
  }

  async [_alloc](source) {
    const { malloc, memcpy } = this[_mem];
    this.#size = source.byteLength;
    this.#ptr = await malloc(this.#size);

    if (source instanceof Uint8Array) {
      await memcpy(this.#ptr, source);
    }

    return this;
  }

  async valueOf() {
    const { memcpy } = this[_mem];

    return new Uint8Array(await memcpy(null, this.#ptr, this.#size));
  }
}

class Allocator {
  constructor(mem) {
    this[_mem] = mem;
  }

  get memcpy() {
    return this[_mem].memcpy;
  }

  get malloc() {
    return this[_mem].malloc;
  }

  get types() {
    const mem = this[_mem];
    const { memcpy } = mem;

    return {
      ptr(source) {
        return source;
      },
      async box(source) {
        const boxed = new Box();
        boxed[_mem] = mem;
        await boxed[_alloc](source);

        return boxed;
      },
      capacity(byteLength) {
        return { byteLength };
      },
      u32(source) {
        return memcpy(null, +source, 4).then((data) =>
          new DataView(data.buffer).getUint32(0, true),
        );
      },
    };
  }

  get globals() {
    return this[_mem].globals;
  }
}

export function withAllocator(fn) {
  return function (exports, memoryAccessor) {
    const { malloc, free } = exports;
    const accessor = { malloc, free, ...memoryAccessor };
    const allocator = new Allocator(accessor);

    return fn(exports, allocator);
  };
}
