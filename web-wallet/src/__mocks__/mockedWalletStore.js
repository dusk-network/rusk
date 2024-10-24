import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

import { mockReadableStore } from "$lib/dusk/test-helpers";

const seed = new Uint8Array(64);
const seeder = () => seed;
const profileGenerator = new ProfileGenerator(seeder);
const profiles = [
  await profileGenerator.default,
  await profileGenerator.next(),
  await profileGenerator.next(),
];
const currentProfile = profiles[0];
const shielded = { spendable: 50_000_000_000_000n, value: 2_345_000_000_000n };

/** @type {WalletStoreContent} */
const content = {
  balance: { shielded },
  currentProfile,
  initialized: true,
  profiles,
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
