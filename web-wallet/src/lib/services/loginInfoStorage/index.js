import {
	compose,
	isNull,
	mapValuesWith,
	unless
} from "lamb";

import { base64ToBytes,	bytesToBase64 } from "$lib/dusk/base64";

const storeKey = `${CONFIG.LOCAL_STORAGE_APP_KEY}-login`;
const fromStorageString = unless(
	isNull,
	compose(mapValuesWith(base64ToBytes), JSON.parse)
);
const toStorageString = compose(JSON.stringify, mapValuesWith(bytesToBase64));

const loginInfoStorage = {
	/** @returns {MnemonicEncryptInfo | null} */
	get () {
		return fromStorageString(localStorage.getItem(storeKey));
	},

	remove () {
		localStorage.removeItem(storeKey);
	},

	/** @param {MnemonicEncryptInfo} info */
	set (info) {
		localStorage.setItem(storeKey, toStorageString(info));
	}
};

export default loginInfoStorage;
