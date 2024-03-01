/**
 * @param {String} pwd
 * @returns {Promise<CryptoKey>}
 */
const getKeyMaterial = (pwd) =>
  crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(pwd),
    "PBKDF2",
    false,
    ["deriveBits", "deriveKey"]
  );

/**
 * @param {String} pwd
 * @param {Uint8Array} salt
 * @returns {Promise<CryptoKey>}
 */
const getDerivedKey = async (pwd, salt) =>
  crypto.subtle.deriveKey(
    {
      hash: "SHA-256",
      iterations: 10000,
      name: "PBKDF2",
      salt,
    },
    await getKeyMaterial(pwd),
    { length: 256, name: "AES-GCM" },
    true,
    ["encrypt", "decrypt"]
  );

export default getDerivedKey;
