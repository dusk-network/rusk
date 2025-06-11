import { hexToBytes, isValidHex } from "./";

/**
 * Decodes a hexadecimal string to utf-8 and formats it if it's a JSON string
 *
 * @param {string} value
 * @returns {string}
 */
function decodeHexString(value) {
  if (!isValidHex(value)) {
    return value;
  }

  const bytes = hexToBytes(value);
  const decoder = new TextDecoder();
  let decodedString;
  try {
    decodedString = decoder.decode(bytes);
  } catch {
    return value;
  }

  //Check whether the string contains almost all visible Unicode characters up to the end of the BMP
  /* eslint-disable-next-line no-control-regex */
  if (/^[\u0000-\u007F\u00A0-\uFFFF]*$/.test(decodedString)) {
    try {
      return JSON.stringify(JSON.parse(decodedString), null, 2);
    } catch {
      return decodedString;
    }
  }

  return value;
}

export default decodeHexString;
