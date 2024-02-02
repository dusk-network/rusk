import { mockReadableStore } from "$lib/dusk/test-helpers";
import { addresses } from "$lib/mock-data";

const balance = { maximum: 50000, value: 2345 };
const currentAddress = addresses[0];

/** @type {import("$lib/stores/stores").WalletStoreContent} */
const content = {
	addresses,
	balance,
	currentAddress,
	error: null,
	initialized: true,
	isSyncing: false
};

export default mockReadableStore(content);
