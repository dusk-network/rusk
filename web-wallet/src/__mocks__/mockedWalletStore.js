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
const unshielded = { nonce: 1234n, value: shielded.value / 2n };

/** @type {WalletStoreContent} */
const content = {
  balance: { shielded, unshielded },
  currentProfile,
  initialized: true,
  profiles,
  syncStatus: {
    error: null,
    from: 0n,
    isInProgress: false,
    last: 0n,
    progress: 0,
  },
};

const mockedWalletStore = mockReadableStore(content);

export default mockedWalletStore;
