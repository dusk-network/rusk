
import { Wallet } from "@dusk-network/dusk-wallet-js";

/**
 * Gets a `Wallet` instance.
 * @param {Uint8Array} seed
 * @param {Number} [gasLimit=2900000000]
 * @param {Number} [gasPrice=1]
 * @returns {Wallet}
 */

const getWallet = (seed, gasLimit, gasPrice) => new Wallet(Array.from(seed), gasLimit, gasPrice);

export default getWallet;
