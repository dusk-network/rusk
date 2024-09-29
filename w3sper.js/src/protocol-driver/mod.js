// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as exu from "https://rawcdn.githack.com/dusk-network/exu/v0.1.2/src/mod.js";
import { none } from "./none.js";

import { DriverError } from "./error.js";
import * as DataBuffer from "./buffer.js";
import { withAllocator } from "./alloc.js";

const rng = () => crypto.getRandomValues(new Uint8Array(32));

const uninit = Object.freeze([
  none`No Protocol Driver loaded yet. Call "load" first.`,
  none`No size set yet. Load the Protocol Driver first.`,
]);

let [protocolDriverModule, driverEntrySize] = uninit;

export const getEntrySize = () => driverEntrySize;

export function load(source, importsURL) {
  // If the module is already loaded, no need to load it again.
  if (!(protocolDriverModule instanceof none)) {
    return;
  }

  protocolDriverModule = new exu.Module(source);

  if (importsURL instanceof URL) {
    protocolDriverModule.defaultImports = importsURL;
  }

  // Parse known globals once.

  driverEntrySize = protocolDriverModule.task(
    withAllocator(async function (_exports, allocator) {
      const { ptr, u32 } = allocator.types;
      const { globals } = allocator;

      const key = await u32(ptr(globals.KEY_SIZE));
      const item = await u32(ptr(globals.ITEM_SIZE));

      return { key, item };
    }),
  )();
}

export function unload() {
  if (protocolDriverModule instanceof none || driverEntrySize instanceof none) {
    return Promise.resolve();
  } else {
    return Promise.all([protocolDriverModule, driverEntrySize]).then(() => {
      [protocolDriverModule, driverEntrySize] = uninit;
    });
  }
}

export async function bookmarks(notes) {
  const task = protocolDriverModule.task(async function (
    { malloc, bookmarks },
    { memcpy },
  ) {
    if (notes.length === 0) {
      return [];
    }

    // Prepare the notes buffer
    let notesBuffer = new Uint8Array(DataBuffer.from(notes.values()));

    // Allocate memory for the notes + 4 bytes for the length
    let ptr = await malloc(notesBuffer.byteLength);

    // Copy the notes to the WASM memory
    await memcpy(ptr, notesBuffer, notesBuffer.byteLength);

    let bookmarks_ptr = await malloc(8);

    let code = await bookmarks(ptr, bookmarks_ptr);
    if (code > 0) throw DriverError.from(code);
    bookmarks_ptr = new DataView(
      (await memcpy(null, bookmarks_ptr, 4)).buffer,
    ).getUint32(0, true);

    return await memcpy(null, bookmarks_ptr, notes.size * 8);
  });

  return await task();
}

export async function pickNotes(owner, notes, value) {
  const task = protocolDriverModule.task(async function (
    { malloc, pick_notes },
    { memcpy },
  ) {
    if (notes.length === 0) {
      return new Map();
    }

    // Copy the seed to avoid invalidating the original buffer
    let seed = new Uint8Array(await owner.seed);
    // Copy the seed to the WASM memory
    let seed_ptr = await malloc(64);
    await memcpy(seed_ptr, seed, 64);

    // Prepare the notes buffer
    let notesBuffer = new Uint8Array(DataBuffer.from(notes.entries()));

    // Allocate memory for the notes + 4 bytes for the length
    let ptr = await malloc(notesBuffer.byteLength);

    // Copy the notes to the WASM memory
    await memcpy(ptr, notesBuffer, notesBuffer.byteLength);

    // Copy the value to the WASM memory
    let valueBuffer = new Uint8Array(8);
    new DataView(valueBuffer.buffer).setBigUint64(0, value, true);
    let value_ptr = await malloc(valueBuffer.length);
    await memcpy(value_ptr, valueBuffer, valueBuffer.byteLength);

    let code = await pick_notes(seed_ptr, +owner, value_ptr, ptr);
    if (code > 0) throw DriverError.from(code);

    let len = new DataView((await memcpy(null, ptr, 4)).buffer).getUint32(
      0,
      true,
    );

    notesBuffer = await memcpy(null, ptr + 4, len);

    let notesLen = new DataView(notesBuffer.buffer).getUint32(
      notesBuffer.byteLength - 4,
      true,
    );

    let itemSize = (notesBuffer.buffer.byteLength - 8) / notesLen;
    let keySize = 32;
    let valueSize = itemSize - keySize;

    let result = new Map();
    for (let i = 0; i < itemSize * notesLen; i += itemSize) {
      let key = new Uint8Array(keySize);
      let value = new Uint8Array(valueSize);
      key.set(notesBuffer.subarray(i, i + keySize));
      value.set(notesBuffer.subarray(i + keySize, i + itemSize));

      result.set(key, value);
    }

    return result;
  });

  return await task();
}

export const generateProfile = (seed, n) =>
  protocolDriverModule.task(
    withAllocator(async function ({ generate_profile }, allocator) {
      const { box, capacity } = allocator.types;

      // Allocates memory on the WASM heap and then places `seed` into it.
      // We copy the seed since we do not want to transfer the original buffer
      // over the WASM memory.
      let seed_ptr = await box(seed.slice(0));

      // Allocates memory on the WASM heap for the profile
      let out = await box(capacity(64 + 96));

      await generate_profile(+seed_ptr, n, +out);

      // Return the content of the `out` boxed value
      return out.valueOf();
    }),
  )();

export const mapOwned = (owners, notes) =>
  protocolDriverModule.task(
    withAllocator(async function ({ map_owned }, allocator) {
      const { box, capacity, u64, u32x, ptrx, databuffer } = allocator.types;

      if (owners.length === 0) {
        return new Map();
      }

      const firstOwner = owners[0];
      const sharesSameSource = owners.every((owner) =>
        firstOwner.sameSourceOf(owner),
      );

      if (!sharesSameSource) {
        throw new Error("All owners must be generated from the same source");
      }

      let { key: keySize, item: itemSize } = await driverEntrySize;
      let entrySize = keySize + itemSize;

      let notesBuffer = new Uint8Array(
        DataBuffer.from(notes, { size: notes.byteLength / itemSize }),
      );

      // Allocates memory on the WASM heap and then places `seed` into it.
      // We copy the seed since we do not want to transfer the original buffer
      // over the WASM memory.
      let seed = await box((await firstOwner.seed).slice(0));

      // Allocate memory for the notes + 4 bytes for the length
      // let ptr = await malloc(notesBuffer.byteLength);

      // Copy the notes to the WASM memory
      //
      // await memcpy(ptr, notesBuffer, notesBuffer.byteLength);
      let notes_ptr = await box(notesBuffer);

      // Convert the profile to indexes and copy them to a Uint8Array
      let indexes = new Uint8Array(owners.length + 1);
      indexes[0] = owners.length;
      indexes.set(
        owners.map((p) => +p),
        1,
      );

      let idx_ptr = await box(indexes);

      let out_ptr = await box(u32x);

      let info_ptr = await box(capacity(16)); //malloc(16);

      let code = await map_owned(
        +seed,
        +idx_ptr,
        +notes_ptr,
        +out_ptr,
        +info_ptr,
      );
      if (code > 0) throw DriverError.from(code);

      out_ptr = await out_ptr.valueOf();
      // let len = await u32x(out_ptr);

      let [buff] = await databuffer(out_ptr);

      const { size, totalLength } = DataBuffer.layout(buff);

      const sizes = [];
      for (let i = totalLength; i > 0; i -= totalLength / size) {
        const vecLayout = DataBuffer.layout(buff.slice(0, -i));
        sizes.push(vecLayout.size);
      }

      notesBuffer = buff;
      let totalLen = buff.byteLength - (size + 1) * 8;

      let blockHeight = await u64(+info_ptr);
      let bookmark = await u64(+info_ptr + 8);

      let results = sizes.map((_) => new Map());
      let j = 0;
      for (let i = 0; i < totalLen; i += entrySize) {
        let key = new Uint8Array(keySize);
        let value = new Uint8Array(itemSize);
        key.set(notesBuffer.subarray(i, i + keySize));
        value.set(notesBuffer.subarray(i + keySize, i + entrySize));

        while (j < sizes.length && sizes[j] === 0) j++;
        results[j].set(key, value);
        sizes[j]--;
      }

      return [results, { blockHeight, bookmark }];
    }),
  )();

export async function balance(seed, n, notes) {
  const task = await protocolDriverModule.task(async function (
    { malloc, balance },
    { memcpy },
  ) {
    // Copy the seed to avoid invalidating the original buffer
    seed = new Uint8Array(seed);

    let seed_ptr = await malloc(64);
    await memcpy(seed_ptr, seed, 64);

    let notesBuffer = new Uint8Array(DataBuffer.from(notes.values()));

    let ptr = await malloc(notesBuffer.byteLength);
    await memcpy(ptr, notesBuffer);
    let info_ptr = await malloc(16);

    const _result = await balance(seed_ptr, n, ptr, info_ptr);

    let info = new Uint8Array(await memcpy(null, info_ptr, 16));

    let value = new DataView(info.buffer).getBigUint64(0, true);
    let spendable = new DataView(info.buffer).getBigUint64(8, true);

    return { value, spendable };
  });
  return await task();
}

export const accountsIntoRaw = async (users) =>
  protocolDriverModule.task(async function (
    { malloc, accounts_into_raw },
    { memcpy },
  ) {
    let buffer = new Uint8Array(
      DataBuffer.from(
        DataBuffer.flatten(users.map((user) => user.account.valueOf())),
      ),
    );

    // copy buffer into WASM memory
    let ptr = await malloc(buffer.byteLength);
    await memcpy(ptr, buffer);

    // allocate pointer for result
    let out_ptr = await malloc(4);

    // call the WASM function
    const code = await accounts_into_raw(ptr, out_ptr);
    if (code > 0) throw DriverError.from(code);

    // Copy the result from WASM memory
    out_ptr = new DataView((await memcpy(null, out_ptr, 4)).buffer).getUint32(
      0,
      true,
    );

    let len = new DataView((await memcpy(null, out_ptr, 4)).buffer).getUint32(
      0,
      true,
    );

    buffer = await memcpy(null, out_ptr + 4, len);
    const size = buffer.byteLength / users.length;

    let result = [];
    for (let i = 0; i < buffer.byteLength; i += size) {
      result.push(new Uint8Array(buffer.subarray(i, i + size)));
    }
    return result;
  })();

export const phoenix = async (info) =>
  protocolDriverModule.task(async function ({ malloc, phoenix }, { memcpy }) {
    const ptr = Object.create(null);

    const seed = new Uint8Array(await info.sender.seed);

    ptr.seed = await malloc(64);
    await memcpy(ptr.seed, seed, 64);

    ptr.rng = await malloc(32);
    await memcpy(ptr.rng, new Uint8Array(rng()));

    const sender_index = +info.sender;
    const receiver = info.receiver.valueOf();
    ptr.receiver = await malloc(receiver.byteLength);
    await memcpy(ptr.receiver, receiver);

    const inputs = DataBuffer.from(info.inputs);

    ptr.inputs = await malloc(inputs.byteLength);
    await memcpy(ptr.inputs, new Uint8Array(inputs));

    const openings = DataBuffer.from(info.openings);
    ptr.openings = await malloc(openings.byteLength);
    await memcpy(ptr.openings, new Uint8Array(openings));

    const root = info.root;
    ptr.root = await malloc(root.byteLength);
    await memcpy(ptr.root, new Uint8Array(root));

    const transfer_value = new Uint8Array(8);
    new DataView(transfer_value.buffer).setBigUint64(
      0,
      info.transfer_value,
      true,
    );
    ptr.transfer_value = await malloc(8);
    await memcpy(ptr.transfer_value, transfer_value);

    const deposit = new Uint8Array(8);
    new DataView(deposit.buffer).setBigUint64(0, info.deposit, true);
    ptr.deposit = await malloc(8);
    await memcpy(ptr.deposit, deposit);

    const gas_limit = new Uint8Array(8);
    new DataView(gas_limit.buffer).setBigUint64(0, info.gas_limit, true);
    ptr.gas_limit = await malloc(8);
    await memcpy(ptr.gas_limit, gas_limit);

    const gas_price = new Uint8Array(8);
    new DataView(gas_price.buffer).setBigUint64(0, info.gas_price, true);
    ptr.gas_price = await malloc(8);
    await memcpy(ptr.gas_price, gas_price);

    // Copy the value to the WASM memory
    const code = await phoenix(
      ptr.rng,
      ptr.seed,
      sender_index,
      ptr.receiver,
      ptr.inputs,
      ptr.openings,
      ptr.root,
      ptr.transfer_value,
      info.obfuscated_transaction,
      ptr.deposit,
      ptr.gas_limit,
      ptr.gas_price,
      info.chainId,
      info.data,
    );
    if (code > 0) throw DriverError.from(code);

    // pub unsafe fn phoenix(
    //     rng: &[u8; 32],
    //     seed: &Seed,
    //     sender_index: u8,
    //     receiver: &[u8; PhoenixPublicKey::SIZE],
    //     inputs: *const u8,
    //     openings: *const u8,
    //     root: &[u8; BlsScalar::SIZE],
    //     transfer_value: *const u64,
    //     obfuscated_transaction: bool, // ?
    //     deposit: *const u64,          // ?
    //     gas_limit: *const u64,
    //     gas_price: *const u64,
    //     chain_id: u8,    // ?
    //     data: *const u8, // ?
  })();
