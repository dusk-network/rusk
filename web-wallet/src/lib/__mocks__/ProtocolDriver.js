/* eslint-disable no-unused-vars */

export async function balance() {
  return { spendable: 0n, value: 0n };
}

/**
 * @param {Uint8Array} seed
 * @param {number} n
 * @returns {Promise<Uint8Array>}
 */
export async function generateProfile(seed, n) {
  return new Uint8Array(64 + 96).fill(99);
}

export async function getMinimumStake() {
  return 1_000_000_000_000n;
}

export function load() {}

export async function unload() {}

export function useAsProtocolDriver() {}
