import { readable } from "svelte/store";

const gasLimits = {
	gasLimitLower: parseInt(import.meta.env.VITE_GAS_LIMIT_LOWER, 10),
	gasLimitUpper: parseInt(import.meta.env.VITE_GAS_LIMIT_UPPER, 10),
	gasPriceLower: parseInt(import.meta.env.VITE_GAS_PRICE_LOWER, 10)
};

const gasStore = readable(gasLimits);

export default gasStore;
