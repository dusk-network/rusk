import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

/**
 * Validates if an input is an account or an address, with feedback on failure or success.
 * @param {String} input The input to validate.
 * @returns {{isValid: boolean, type?: "address" | "account"}} Validation result and the input type.
 *  - `isValid` {Boolean} - true if the input is valid, false if invalid.
 *  - `type?` {"address"|"account"} - The type of input that was validated.
 */
export default function validateAddress(input) {
  const type = ProfileGenerator.typeOf(input);

  if (type === "account" || type === "address") {
    return {
      isValid: true,
      type,
    };
  }

  return { isValid: false };
}
