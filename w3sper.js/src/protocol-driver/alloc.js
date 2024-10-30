// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as DataBuffer from "./buffer.js";
import { none } from "./none.js";

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

    const box = async (source) => {
      const boxed = new Box();
      boxed[_mem] = mem;
      await boxed[_alloc](source);

      if (typeof source === "function") {
        boxed.valueOf = source;
      }

      return boxed;
    };

    function u32x(source) {
      source ??= this;
      return memcpy(null, +source, 4).then((data) =>
        new DataView(data.buffer).getUint32(0, true),
      );
    }
    u32x.byteLength = 4;

    function u64(source) {
      source ??= this;
      return memcpy(null, +source, 8).then((data) =>
        new DataView(data.buffer).getBigUint64(0, true),
      );
    }
    u32x.byteLength = 8;

    function ptrx(type) {
      return async function () {
        let address = await u32x.call(this);
        return await type.call(address);
      };
    }
    ptrx.byteLength = u32x.byteLength;

    async function databuffer(source) {
      source ??= this;
      const len = await u32x(source);
      const data = await memcpy(null, source + 4, len);

      let align = new DataView(data.buffer).getUint32(
        data.byteLength - 8,
        true,
      );

      let size = new DataView(data.buffer).getUint32(data.byteLength - 4, true);

      return [data, { align, size }];
    }

    return {
      ptr(source) {
        return source;
      },

      box,
      u64,
      u32x,
      ptrx,
      databuffer,

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
