import { getKey } from "lamb";
import { Gas } from "@dusk/w3sper";

import { walletStore } from "$lib/stores";
import wasmPath from "$lib/vendor/standard_bridge_dd_opt.wasm?url";

const VITE_EVM_BRIDGE_ID = import.meta.env.VITE_EVM_BRIDGE_ID;

/** @type {(amount: bigint, gasPrice: bigint, gasLimit: bigint) => Promise<string>} */
const executeEvmBridgeDeposit = (amount, gasPrice, gasLimit) => {
  return walletStore
    .contractFunctionCall(
      amount,
      new Gas({
        limit: gasLimit,
        price: gasPrice,
      }),
      VITE_EVM_BRIDGE_ID,
      "deposit",
      {
        amount: 200,
        // eslint-disable-next-line camelcase
        extra_data: [],
        fee: "200000",
        to: "",
      },
      wasmPath
    )
    .then(getKey("hash"));
};

export default executeEvmBridgeDeposit;
