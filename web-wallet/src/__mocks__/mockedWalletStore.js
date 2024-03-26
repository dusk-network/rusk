import { mockReadableStore } from "$lib/dusk/test-helpers";
import { addresses } from "$lib/mock-data";

const balance = { maximum: 50000, value: 2345 };
const currentAddress = addresses[0];

/** @type {WalletStoreContent} */
const content = {
  addresses,
  balance,
  currentAddress,
  initialized: true,
  syncStatus: { current: 0, error: null, isInProgress: false, last: 0 },
};

const mockedWalletStore = mockReadableStore(content);

export default mockedWalletStore;
