import getDerivedKey from "./getDerivedKey";

/**
 * @param {BufferSource} buffer
 * @param {string} pwd
 * @returns {Promise<WalletEncryptInfo>}
 */
async function encryptBuffer(buffer, pwd) {
  const salt = crypto.getRandomValues(new Uint8Array(32));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const key = await getDerivedKey(pwd, salt);
  const data = new Uint8Array(
    await crypto.subtle.encrypt({ iv, name: "AES-GCM" }, key, buffer)
  );

  return { data, iv, salt };
}

export default encryptBuffer;
