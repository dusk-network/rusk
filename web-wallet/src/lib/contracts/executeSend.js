import { getKey } from "lamb";
import { Gas } from "@dusk/w3sper";

import { getBlocklistedRecipient } from "$lib/security/addressBlocklist";
import { walletStore } from "$lib/stores";

/** @type {(to: string, amount: bigint, memo: string, gasPrice: bigint, gasLimit: bigint) => Promise<string>} */
const executeSend = (to, amount, memo, gasPrice, gasLimit) => {
  const blocked = getBlocklistedRecipient(to);

  if (blocked) {
    return Promise.reject(new Error(blocked.reason));
  }

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
