// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as exu from "https://rawcdn.githack.com/dusk-network/exu/v0.1.2/src/mod.js";
// import * as exu from "../../../exu/src/mod.js";
import { none } from "./none.js";

import { DriverError } from "./protocol-driver/error.js";
import * as DataBuffer from "./protocol-driver/buffer.js";

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
  driverEntrySize = protocolDriverModule.task(async function (
    { malloc },
    { memcpy, globals },
  ) {
    let data = await memcpy(null, globals.KEY_SIZE, 4);
    let key = new DataView(data.buffer).getUint32(0, true);
    data = await memcpy(null, globals.ITEM_SIZE, 4);
    let item = new DataView(data.buffer).getUint32(0, true);

    return { key, item };
  })();
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

export async function generateProfile(seed, n) {
  const task = protocolDriverModule.task(async function (
    { malloc, generate_profile },
    { memcpy },
  ) {
    // Copy the seed to avoid invalidating the original buffer
    seed = new Uint8Array(seed);

    // Copy the seed to the WASM memory
    let seed_ptr = await malloc(64);
    await memcpy(seed_ptr, seed, 64);

    // Allocate memory for the profile
    let ptr = await malloc(64 + 96);
    await generate_profile(seed_ptr, n, ptr);

    // Copy the profile to a new buffer
    return new Uint8Array(await memcpy(null, ptr, 64 + 96));
  });

  return await task();
}

export async function mapOwned(owners, notes) {
  const task = protocolDriverModule.task(async function (
    { malloc, map_owned },
    { memcpy },
  ) {
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

    // Copyw the seed to avoid invalidating the original buffer
    let seed = new Uint8Array(await firstOwner.seed);
    // Copy the seed to the WASM memory
    let seed_ptr = await malloc(64);
    await memcpy(seed_ptr, seed, 64);

    // Allocate memory for the notes + 4 bytes for the length
    let ptr = await malloc(notesBuffer.byteLength);

    // Copy the notes to the WASM memory
    //
    await memcpy(ptr, notesBuffer, notesBuffer.byteLength);

    // Convert the profile to indexes and copy them to a Uint8Array
    let indexes = new Uint8Array(owners.length + 1);
    indexes[0] = owners.length;
    indexes.set(
      owners.map((p) => +p),
      1,
    );

    let idx = await malloc(indexes.byteLength);
    await memcpy(idx, indexes, indexes.byteLength);

    let out_ptr = await malloc(4);

    let info_ptr = await malloc(16);

    let code = await map_owned(seed_ptr, idx, ptr, out_ptr, info_ptr);
    if (code > 0) throw DriverError.from(code);

    out_ptr = new DataView((await memcpy(null, out_ptr, 4)).buffer).getUint32(
      0,
      true,
    );

    let len = new DataView((await memcpy(null, out_ptr, 4)).buffer).getUint32(
      0,
      true,
    );

    notesBuffer = await memcpy(null, out_ptr + 4, len);

    let notesLen = new DataView(notesBuffer.buffer).getUint32(
      notesBuffer.byteLength - 4,
      true,
    );

    let info = new Uint8Array(await memcpy(null, info_ptr, 16));

    let blockHeight = new DataView(info.buffer).getBigUint64(0, true);
    let bookmark = new DataView(info.buffer).getBigUint64(8, true);

    let result = new Map();
    for (let i = 0; i < entrySize * notesLen; i += entrySize) {
      let key = new Uint8Array(keySize);
      let value = new Uint8Array(itemSize);
      key.set(notesBuffer.subarray(i, i + keySize));
      value.set(notesBuffer.subarray(i + keySize, i + entrySize));

      result.set(key, value);
    }

    return [result, { blockHeight, bookmark }];
  });

  return await task();
}

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

export async function accountsIntoRaw(users) {
  const task = await protocolDriverModule.task(async function (
    { malloc, accounts_into_raw },
    { memcpy },
  ) {});

  return await task();
}

export async function bookmarkFrom(note) {
  const task = await protocolDriverModule.task(async function (
    { malloc, bookmark },
    { memcpy },
  ) {
    let data = new Uint8Array(note.byteLength + 4);
    new DataView(data.buffer).setUint32(0, note.byteLength, true);
    data.set(note, 4);

    let ptr = await malloc(data.length);
    await memcpy(ptr, data, data.length);
    let bookmark_ptr = await malloc(8);

    const result = await bookmark(ptr, bookmark_ptr);

    return await memcpy(null, bookmark_ptr, 8);
  });
  return await task();
}
