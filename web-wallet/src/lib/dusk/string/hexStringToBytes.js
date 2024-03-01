/**
 * Converts a hexadecimal string to a `Uint8Array`.
 * @param {String} s
 * @returns {Uint8Array}
 */
const hexStringToBytes = (s) =>
  Uint8Array.from(s.match(/.{1,2}/g) ?? [], (hexByte) => parseInt(hexByte, 16));

export default hexStringToBytes;
