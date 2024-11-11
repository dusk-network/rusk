const scaleFactor = 1e9;

/**
 * @param {number} n
 * @returns {bigint}
 */
const duskToLux = (n) =>
  BigInt(Math.floor(n)) * BigInt(scaleFactor) +
  BigInt(Math.floor((n % 1) * scaleFactor));

export default duskToLux;
