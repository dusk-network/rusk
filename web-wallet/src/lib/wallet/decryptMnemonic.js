import getDerivedKey from "./getDerivedKey";

/**
 * @param {MnemonicEncryptInfo} mnemonicEncryptInfo
 * @param {String} pwd
 * @returns {Promise<String>}
 */
async function decryptMnemonic (mnemonicEncryptInfo, pwd) {
	const { data, iv, salt } = mnemonicEncryptInfo;
	const key = await getDerivedKey(pwd, salt);
	const plaintext = await crypto.subtle.decrypt({ iv, name: "AES-GCM" }, key, data);

	return new TextDecoder().decode(plaintext);
}

export default decryptMnemonic;
