const scaleFactorAsNumber = 1e9;
const scaleFactorAsBigInt = BigInt(scaleFactorAsNumber);

/**
 * Temporary conversion: in the near future
 * we will accept only BigInt as input.
 *
 * @param {bigint} n
 * @returns {number}
 */
function luxToDusk(n) {
  const integerPart = Number(n / scaleFactorAsBigInt);
  const decimalPart = Number(n % scaleFactorAsBigInt);

  return integerPart + decimalPart / scaleFactorAsNumber;
}

export default luxToDusk;
