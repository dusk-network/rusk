import { isType } from "lamb";

/** @type {(n: any) => n is number} */
const isNumber = isType("Number");

const scaleFactorAsNumber = 1e9;
const scaleFactorAsBigInt = BigInt(scaleFactorAsNumber);

/**
 * Temporary conversion function until
 * `dusk-wallet-js` exposes its own.
 *
 * @param {bigint | number} n
 * @returns {number}
 */
function luxToDusk(n) {
  if (isNumber(n)) {
    return n / scaleFactorAsNumber;
  } else {
    const integerPart = Number(n / scaleFactorAsBigInt);
    const decimalPart = Number(n % scaleFactorAsBigInt);

    return integerPart + decimalPart / scaleFactorAsNumber;
  }
}

export default luxToDusk;
