import { mockReadableStore } from "$lib/dusk/test-helpers";
import { addresses } from "$lib/mock-data";

const shielded = { spendable: 50_000_000_000_000n, value: 2_345_000_000_000n };
const currentAddress = addresses[0];

/** @type {WalletStoreContent} */
const content = {
  addresses,
  balance: { shielded },
  currentAddress,
  currentProfile: null,
  initialized: true,
  profiles: [],
  syncStatus: {
    current: 0n,
    error: null,
    isInProgress: false,
    last: 0n,
    progress: 0,
  },
};

const mockedWalletStore = mockReadableStore(content);

export default mockedWalletStore;
