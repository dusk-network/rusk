import loginInfoStorage from "$lib/services/loginInfoStorage";
import { encryptMnemonic } from "$lib/wallet";

/**
 * @param {string[]} mnemonicPhrase
 * @param {string} password
 */
async function refreshLocalStoragePasswordInfo(mnemonicPhrase, password) {
  loginInfoStorage.remove();

  if (password.length !== 0) {
    const mnemonic = mnemonicPhrase.join(" ");
    const encryptedData = await encryptMnemonic(mnemonic, password);

    loginInfoStorage.set(encryptedData);
  }
}

export default refreshLocalStoragePasswordInfo;
