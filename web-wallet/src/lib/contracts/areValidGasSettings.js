import { gasStore } from "$lib/stores";
import { isType } from "lamb";
import { get } from "svelte/store";

/**
 *
 * @param {bigint} price
 * @param {bigint} limit
 * @returns {Boolean}
 */
const areValidGasSettings = (price, limit) => {
  const gasLimits = get(gasStore);
  let isValidPrice = false;
  let isValidLimit = false;
  let isGasValid = false;

  if ([price, limit].every(isType("BigInt"))) {
    isValidPrice = price >= gasLimits.gasPriceLower && price <= limit;
    isValidLimit =
      limit >= gasLimits.gasLimitLower && limit <= gasLimits.gasLimitUpper;
    isGasValid = isValidPrice && isValidLimit;
  }

  return isGasValid;
};

export default areValidGasSettings;
