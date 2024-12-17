import { readable } from "svelte/store";

const gasLimits = {
  gasLimitLower: BigInt(import.meta.env.VITE_GAS_LIMIT_LOWER ?? 10_000_000),
  gasLimitUpper: BigInt(import.meta.env.VITE_GAS_LIMIT_UPPER ?? 1_000_000_000),
  gasPriceLower: BigInt(import.meta.env.VITE_GAS_PRICE_LOWER ?? 1),
};

const gasStore = readable(gasLimits);

export default gasStore;
