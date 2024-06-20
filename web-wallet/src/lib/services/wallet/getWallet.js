import { Wallet } from "@dusk-network/dusk-wallet-js";

/**
 * Gets a `Wallet` instance.
 * @param {Uint8Array} seed
 * @returns {Wallet}
 */

const getWallet = (seed) => new Wallet(Array.from(seed));

export default getWallet;
