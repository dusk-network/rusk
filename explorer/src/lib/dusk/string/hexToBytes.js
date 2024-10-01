import { isValidHex } from "./";

/**
 * Transform a hexadecimal string to a Uint8Array of bytes
 *
 * @param {string} value
 * @returns {Uint8Array}
 */
const hexToBytes = (value) => {
  if (!isValidHex(value)) {
    throw new Error(`Given value "${value}" is not a valid hex string`);
  }

  value = value.replace(/^0x/i, "");
  const bytes = new Uint8Array(value.length / 2);
  for (let i = 0; i < value.length; i += 2) {
    bytes[i / 2] = parseInt(value.slice(i, i + 2), 16);
  }
  return bytes;
};

export default hexToBytes;
