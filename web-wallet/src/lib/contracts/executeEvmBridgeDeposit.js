import { getKey } from "lamb";
import { Gas } from "@dusk/w3sper";

import { walletStore } from "$lib/stores";
import wasmPath from "$lib/vendor/standard_bridge_dd_opt.wasm?url";

const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;

/**
 * @param {string} hex
 */
function hexToBytes(hex) {
  const s = hex.startsWith("0x") ? hex.slice(2) : hex;
  if (s.length !== 40 || !/^[0-9a-fA-F]+$/.test(s)) {
    throw new Error("Must be 20-byte hex (40 chars)");
  }
  const out = new Array(20);
  for (let i = 0; i < 20; i++) out[i] = parseInt(s.slice(i * 2, i * 2 + 2), 16);
  return out; // number[]
}

/** @type {(to: string, deposit: bigint, gasPrice: bigint, gasLimit: bigint) => Promise<string>} */
const executeEvmBridgeDeposit = (to, deposit, gasPrice, gasLimit) => {
  const fee = BigInt(500000);
  const amount = deposit;
  const extraData = Array.from(new Uint8Array());

  return walletStore
    .contractFunctionCall(
      deposit + fee,
      new Gas({ limit: Number(gasLimit), price: Number(gasPrice) }),
      VITE_BRIDGE_CONTRACT_ID,
      "deposit",
      [hexToBytes(to), Number(amount), Number(fee), extraData],
      wasmPath
    )
    .then(getKey("hash"));
};

export default executeEvmBridgeDeposit;
