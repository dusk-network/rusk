/**
 * Truncates the decimal portion of a numeric string to DUSK_DECIMALS places.
 * @param {string} numberAsString - The numeric string to truncate.
 * @returns {string} The truncated numeric string.
 * @param {number} decimalPlaces
 */
function truncateDecimals(numberAsString, decimalPlaces) {
  if (typeof numberAsString !== "string") {
    throw new TypeError("Expected a string.");
  }

  // Trim any whitespace
  const trimmed = numberAsString.trim();

  // Determine the decimal separator by checking which one appears and handling a possible thousands separator scenario
  const dotIndex = trimmed.indexOf(".");
  const commaIndex = trimmed.indexOf(",");

  // If both separators appear, decide which one is the decimal based on which appears last
  let separator;
  if (dotIndex > -1 && commaIndex > -1) {
    separator = Math.max(dotIndex, commaIndex) === dotIndex ? "." : ",";
  } else {
    separator = dotIndex > -1 ? "." : commaIndex > -1 ? "," : "";
  }

  if (!separator) {
    return trimmed;
  }

  const [integer, decimal] = trimmed.split(separator);
  return decimal
    ? `${integer}${separator}${decimal.slice(0, decimalPlaces)}`
    : trimmed;
}

export default truncateDecimals;
