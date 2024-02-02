import getDerivedKey from "./getDerivedKey";

/**
 * @param {String} mnemonic
 * @param {String} pwd
 * @returns {Promise<MnemonicEncryptInfo>}
 */
async function encryptMnemonic (mnemonic, pwd) {
	const plaintext = new TextEncoder().encode(mnemonic);
	const salt = crypto.getRandomValues(new Uint8Array(32));
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const key = await getDerivedKey(pwd, salt);
	const data = new Uint8Array(await crypto.subtle.encrypt({ iv, name: "AES-GCM" }, key, plaintext));

	return { data, iv, salt };
}

export default encryptMnemonic;
