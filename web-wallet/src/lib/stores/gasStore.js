import { readable } from "svelte/store";

const gasLimits = {
  gasLimitLower: BigInt(import.meta.env.VITE_GAS_LIMIT_LOWER ?? 10000000),
  gasLimitUpper: BigInt(import.meta.env.VITE_GAS_LIMIT_UPPER ?? 1000000000),
  gasPriceLower: BigInt(import.meta.env.VITE_GAS_PRICE_LOWER ?? 1),
};

const gasStore = readable(gasLimits);

export default gasStore;
