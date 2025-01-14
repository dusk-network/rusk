import decryptBuffer from "./decryptBuffer";

/**
 * @param {WalletEncryptInfo} encryptInfo
 * @param {string} pwd
 * @returns {Promise<string>}
 */
const decryptMnemonic = async (encryptInfo, pwd) =>
  new TextDecoder().decode(await decryptBuffer(encryptInfo, pwd));

export default decryptMnemonic;
