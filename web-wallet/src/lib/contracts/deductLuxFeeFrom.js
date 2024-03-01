import { duskToLux, luxToDusk } from "$lib/dusk/currency";

/**
 * Deducts a fee in Lux from an amount in Dusk
 * and returns a value in Dusk.
 * If the returned value is negative, the fee exceeds
 * the given amount.
 *
 * @param {number} duskAmount
 * @param {number} luxFee
 * @returns {number}
 */
const deductLuxFeeFrom = (duskAmount, luxFee) =>
  +luxToDusk(duskToLux(duskAmount) - luxFee).toFixed(9);

export default deductLuxFeeFrom;
