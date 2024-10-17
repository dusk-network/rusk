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

// function calculateAllocations(entries) {
//   let offset = 0;
//   const allocations = new Map();

//   for (let [name, value] of entries) {
//     value = value?.valueOf();
//     if (value?.byteLength) {
//       value = DataBuffer.from(value);
//       allocations.set(name, { offset, value });
//       offset += value.byteLength;
//       continue;
//     } else if (typeof value?.[Symbol.iterator] === "function") {
//       value = DataBuffer.from(value);
//       allocations.set(name, { offset, value });
//       offset += value.byteLength;
//       continue;
//     } else if (typeof value === "bigint") {
//       const buffer = new ArrayBuffer(8);
//       new DataView(buffer).setBigUint64(0, value, true);
//       allocations.set(name, { offset, value: buffer });
//       offset += 8;
//       continue;
//     }
//   }
//   return [offset, allocations];
// }

// export function withAlloc(fn) {
//   return function (exports, memoryAccessor) {
//     const { malloc } = exports;
//     const accessor = {
//       ...memoryAccessor,
//       allocate: async (object) => {
//         let offset = 0;
//         const offsets = new Map();
//         const [totalSize, allocations] = calculateAllocations(
//           Object.entries(object),
//         );
//         const ptr = await malloc(totalSize);
//         const buffer = new Uint8Array(totalSize);
//         const ptrs = Object.create(null);

//         for (let [name, allocation] of allocations) {
//           const { offset, value } = allocation;
//           buffer.set(value, offset);
//           ptrs[name] = ptr + offset;
//         }
//         await accessor.memcpy(ptr, buffer);
//         return ptrs;
//       },
//     };

//     return fn(exports, accessor);
//   };
// }
