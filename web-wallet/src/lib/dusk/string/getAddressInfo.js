import { ProfileGenerator } from "@dusk/w3sper";

/**
 * Validates if an input is a shielded or an public address, with feedback on failure or success.
 * Additionally, checks for self-referential input.
 * @param {string} input The input to validate.
 * @param {string} shieldedAddress The shielded address to compare.
 * @param {string} publicAddress The public address to compare.
 * @returns {{
 *  isValid: boolean,
 *  type?: "address" | "account",
 *  isSelfReferential?: boolean
 * }} Validation result, the input type, and self-referential status.
 */
export default function getAddressInfo(input, shieldedAddress, publicAddress) {
  const type = ProfileGenerator.typeOf(input);

  if (type === "account" || type === "address") {
    const isSelfReferential =
      (type === "account" && input === publicAddress) ||
      (type === "address" && input === shieldedAddress);

    return {
      isSelfReferential,
      isValid: true,
      type,
    };
  }

  return { isValid: false };
}
