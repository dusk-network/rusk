import { getKey } from "lamb";
import { Gas } from "$lib/vendor/w3sper.js/src/mod";

import { walletStore } from "$lib/stores";

/** @type {(to: string, amount: bigint, gasPrice: bigint, gasLimit: bigint, memo: any) => Promise<string>} */
const executeSend = (to, amount, gasPrice, gasLimit, memo) => {
  return walletStore
    .transfer(
      to,
      amount,
      new Gas({
        limit: gasLimit,
        price: gasPrice,
      }),
      memo
    )
    .then(getKey("hash"));
};

export default executeSend;
