import { getKey } from "lamb";
import { Gas } from "$lib/vendor/w3sper.js/src/mod";

import { walletStore } from "$lib/stores";

/** @type {(to: string, amount: bigint, memo: string, gasPrice: bigint, gasLimit: bigint) => Promise<string>} */
const executeSend = (to, amount, memo, gasPrice, gasLimit) => {
  return walletStore
    .transfer(
      to,
      amount,
      memo,
      new Gas({
        limit: gasLimit,
        price: gasPrice,
      })
    )
    .then(getKey("hash"));
};

export default executeSend;
