import getDerivedKey from "./getDerivedKey";

/**
 * @param {WalletEncryptInfo} encryptInfo
 * @param {string} pwd
 * @returns {Promise<ArrayBuffer>}
 */
async function decryptBuffer(encryptInfo, pwd) {
  const { data, iv, salt } = encryptInfo;
  const key = await getDerivedKey(pwd, salt);

  return await crypto.subtle.decrypt({ iv, name: "AES-GCM" }, key, data);
}

export default decryptBuffer;
