import { getKey } from "lamb";
import { Gas } from "$lib/vendor/w3sper.js/src/mod";

import { walletStore } from "$lib/stores";
import { duskToLux } from "$lib/dusk/currency";

/** @type {(to: string, amount: number, gasPrice: bigint, gasLimit: bigint) => Promise<string>} */
const executeSend = (to, amount, gasPrice, gasLimit) => {
  const luxAmount = duskToLux(amount);

  return walletStore
    .transfer(
      to,
      luxAmount,
      new Gas({
        limit: gasLimit,
        price: gasPrice,
      })
    )
    .then(getKey("hash"));
};

export default executeSend;
