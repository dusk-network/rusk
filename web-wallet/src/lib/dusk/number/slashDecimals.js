/** @type {number} */
const DUSK_DECIMALS = 9;

/**
 * Slashes the number of decimals in a string representation of a number to the number supported by Dusk.
 *
 * @param {string} numberAsString
 * @returns {string}
 */
const slashDecimals = (numberAsString) => {
  const separator = numberAsString.includes(".") ? "." : ",";
  const [integer, decimal] = numberAsString.split(separator);
  return decimal
    ? `${integer}${separator}${decimal.slice(0, DUSK_DECIMALS)}`
    : numberAsString;
};

export default slashDecimals;
