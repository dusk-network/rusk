/**
 * Validates whether the input is a valid EVM address.
 * An EVM address is a 40-character hexadecimal string, optionally prefixed with "0x".
 *
 * @param {string} value - The value to validate
 * @returns {boolean} - True if the value is a valid EVM address
 */
function isValidEvmAddress(value) {
  if (typeof value !== "string") {
    return false;
  }

  const addressWithoutPrefix = value.startsWith("0x") ? value.slice(2) : value;

  // Check if it's exactly 40 characters long and all hexadecimal
  return (
    addressWithoutPrefix.length === 40 &&
    /^[0-9a-fA-F]+$/.test(addressWithoutPrefix)
  );
}

export default isValidEvmAddress;
