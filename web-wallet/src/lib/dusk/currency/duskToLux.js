const scaleFactor = BigInt(1e9);

/**
 * @param {number} n
 * @returns {bigint}
 */
function duskToLux(n) {
  const [integerPart, decimalPart] = n.toString().split(".");

  return (
    BigInt(integerPart) * scaleFactor +
    (decimalPart ? BigInt(decimalPart.padEnd(9, "0")) : 0n)
  );
}

export default duskToLux;
