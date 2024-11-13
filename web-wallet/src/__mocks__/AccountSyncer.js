import { AccountSyncer } from "$lib/vendor/w3sper.js/src/network/syncer/account";

import { stakeInfo } from "$lib/mock-data";

class AccountSyncerMock extends AccountSyncer {
  /**
   * @param {import("$lib/vendor/w3sper.js/src/mod").Network} network
   */
  constructor(network) {
    super(network);
  }

  /**
   * @param {Array<import("$lib/vendor/w3sper.js/src/mod").Profile>} profiles
   * @returns {Promise<AccountBalance[]>}
   */
  async balances(profiles) {
    return Array(profiles.length).fill({
      nonce: 9876n,
      value: 12_345_000_000_000n,
    });
  }

  /**
   * @param {Array<import("$lib/vendor/w3sper.js/src/mod").Profile>} profiles
   * @returns {Promise<StakeInfo[]>}
   */
  async stakes(profiles) {
    return Array(profiles.length).fill(stakeInfo);
  }
}

export default AccountSyncerMock;
