import { getKey } from "lamb";
import { Gas } from "$lib/vendor/w3sper.js/src/mod";

import { walletStore } from "$lib/stores";

const scaleFactor = BigInt(1e9);

/**
 * This `duskToLux` function will replace the one
 * in `$lib/dusk/currency` when the migration to
 * `w3sper.js` advances.
 *
 * @param {number} n
 * @returns {bigint}
 */
function duskToLux(n) {
  const [integerPart, decimalPart] = n.toString().split(".");

  return (
    BigInt(integerPart) * scaleFactor +
    (decimalPart ? BigInt(decimalPart.padEnd(9, "0")) : 0n)
  );
}

/** @type {(to: string, amount: number, gasPrice:number, gasLimit:number) => Promise<string>} */
const executeSend = (to, amount, gasPrice, gasLimit) => {
  const luxAmount = duskToLux(amount);

  return walletStore
    .transfer(
      to,
      luxAmount,
      new Gas({
        limit: BigInt(gasLimit),
        price: BigInt(gasPrice),
      })
    )
    .then(getKey("hash"));
};

export default executeSend;
