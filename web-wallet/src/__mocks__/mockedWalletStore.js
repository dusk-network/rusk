import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/profile";

import { stakeInfo } from "$lib/mock-data";

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
const shieldedBalance = {
  spendable: 50_000_000_000_000n,
  value: 2_345_000_000_000n,
};
const publicBalance = { nonce: 1234n, value: shieldedBalance.value / 2n };

/** @type {WalletStoreContent} */
const content = {
  balance: { publicBalance, shieldedBalance },
  currentProfile,
  initialized: true,
  minimumStake: 1_000_000_000_000n,
  profiles,
  stakeInfo,
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
