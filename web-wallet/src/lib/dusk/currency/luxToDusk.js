const scaleFactorAsNumber = 1e9;
const scaleFactorAsBigInt = BigInt(scaleFactorAsNumber);

/**
 * @param {bigint} n
 * @returns {number}
 */
function luxToDusk(n) {
  const integerPart = Number(n / scaleFactorAsBigInt);
  const decimalPart = Number(n % scaleFactorAsBigInt);

  return integerPart + decimalPart / scaleFactorAsNumber;
}

export default luxToDusk;
