/* eslint-disable no-unused-vars */

/**
 * @param {Uint8Array} seed
 * @param {number} n
 * @returns {Promise<Uint8Array>}
 */
export async function generateProfile(seed, n) {
  return new Uint8Array(64 + 96).fill(99);
}

export function load() {}

export async function unload() {}
