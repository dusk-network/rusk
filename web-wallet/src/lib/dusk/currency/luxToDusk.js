import { isType } from "lamb";

/** @type {(n: any) => n is number} */
const isNumber = isType("Number");

const scaleFactorAsNumber = 1e9;
const scaleFactorAsBigInt = BigInt(scaleFactorAsNumber);

/**
 * Temporary conversion: in the near future
 * we will accept only BigInt as input.
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
