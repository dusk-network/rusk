/**
 * Validates whether the input is a valid hexadecimal string or number.
 *
 * @param {string} value
 * @returns {boolean}
 */
function isValidHex(value) {
  return (
    typeof value === "string" &&
    value.length % 2 === 0 &&
    /^(0x)?[0-9a-f]+$/i.test(value)
  );
}

export default isValidHex;
