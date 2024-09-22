// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/**
 * Calculates and sets layout for the given typed array and size.
 * The typed array buffer should have enough space to store the layout.
 *
 * @param {Uint8Array} dest - The destination typed array where the layout will be applied.
 * @param {number} size - The size to be set in the layout.
 * @returns {ArrayBuffer} - The buffer with the layout applied.
 */
function layout(dest, size) {
  // Subtract 8 bytes to exclude the space reserved for alignment and size
  const totalLength = dest.byteLength - 8;
  // Calculate the alignment value
  const align = 2 ** 32 - ((totalLength + 3) & ~3);

  const view = new DataView(dest.buffer);

  // Set alignment and size at the end of the buffer
  view.setUint32(view.byteLength - 8, align, true);
  view.setUint32(view.byteLength - 4, size, true);
}

/**
 * Allocates a buffer with the specified byte length, plus additional space
 * for storing the buffer's length (4 bytes) and a struct layout (8 bytes).
 *
 * The first 4 bytes of the buffer will store the total byte length (byteLength + 8)
 * in little-endian format.
 *
 * @param {number} byteLength - The byte length for the buffer allocation.
 * @returns {ArrayBuffer} - The allocated buffer with initialized length.
 */
function createBuffer(byteLength, layoutSize) {
  const bufferLength = byteLength + (typeof layoutSize === "number" ? 8 : 0);
  const buffer = new ArrayBuffer(bufferLength + 4);
  const view = new DataView(buffer);

  if (layoutSize > 0) {
    layout(new Uint8Array(buffer, 4), layoutSize);
  }
  // Store the total byte length including the 8-byte layout
  view.setUint32(0, bufferLength, true);

  return buffer;
}

/**
 * Copies the byte contents of each item from the given array into the destination
 * typed array starting at an offset.
 *
 * @param {Uint8Array} dest - The destination typed array where items will be copied into.
 * @param {Uint8Array[]} items - An array of typed arrays to copy into the destination.
 */
function copyInto(dest, items) {
  // Keep track of the current offset in the destination
  let offset = 0;
  for (const item of items) {
    // Copy the item into the destination array at the current offset
    dest.set(item, offset);
    offset += item.byteLength;
  }
}

/**
 * Calculates the total byte length of all items in the given array.
 *
 * @param {Uint8Array[]} items - An array of typed arrays.
 * @returns {number} - The total byte length of all items combined.
 */
const itemsByteLength = (items) =>
  items.reduce((acc, item) => acc + item.byteLength, 0);

/**
 * Flattens a nested array of `Uint8Array` elements into a single `Uint8Array`.
 *
 * @param {Uint8Array | Array<Uint8Array>} item - The input to be flattened.
 *        Can either be a `Uint8Array` or an array of `Uint8Array` objects.
 *        If the input is a `Uint8Array`, it will be returned as-is.

 * @returns {Uint8Array} - A single `Uint8Array` that represents the concatenated
 *        contents of the input. If `item` is already a `Uint8Array`, it will be
 *        returned without modification. Otherwise, the function will merge all
 *        contained `Uint8Array` objects into one continuous `Uint8Array`.
 */
function flatten(item) {
  // If the input is already a Uint8Array, return it as-is.
  if (item instanceof Uint8Array) {
    return item;
  }

  // Otherwise, reduce the array of Uint8Array objects into a single flattened Uint8Array.
  return item.reduce(
    ([acc, n], value) => (acc.set(value, n), [acc, n + value.byteLength]),
    [new Uint8Array(itemsByteLength(item)), 0],
  )[0];
}
/**
 * Creates an ArrayBuffer from the given iterable of typed arrays.
 *
 * This function first consumes the iterable, calculates the total byte length
 * of all the items, allocates a buffer with the appropriate space, and then
 * copies the items into the buffer. After copying, it calculates and sets
 * the layout at the end of the buffer.
 *
 * @param {Iterable<Uint8Array>} iterable - An iterable of typed arrays to be
 *       combined into a single buffer.
 * @returns {ArrayBuffer} - The newly created buffer containing the items and
 *       layout information.
 */
function fromIterable(iterable) {
  // Convert the iterable into an array
  const items = Array.from(iterable, flatten);

  // Calculate the total byte length required for all items
  const byteLength = itemsByteLength(items);

  // Create a buffer with the required length plus space for layout.
  // The layout will be calculated and stored at the end of the buffer
  const buffer = createBuffer(byteLength, items.length);

  // Create a destination Uint8Array, skipping the first 4 bytes for length
  const dest = new Uint8Array(buffer, 4);

  // Copy the items into the destination buffer
  copyInto(dest, items);

  // Return the buffer containing the combined items and layout
  return buffer;
}

function fromBuffer(sourceBuffer, options = {}) {
  const size = options?.size;

  const { byteLength } = sourceBuffer;

  // Create a buffer with the required length plus space for layout.
  // The layout will be calculated and stored at the end of the buffer
  const buffer = createBuffer(byteLength, size);

  // Copy the source buffer into the destination buffer
  new Uint8Array(buffer, 4).set(new Uint8Array(sourceBuffer));

  return buffer;
}

export function from(source, options = {}) {
  if (source instanceof ArrayBuffer) {
    return fromBuffer(source, options);
  } else if (ArrayBuffer.isView(source)) {
    return fromBuffer(source.buffer, options);
  } else if (typeof source[Symbol.iterator] === "function") {
    return fromIterable(source);
  }
}
