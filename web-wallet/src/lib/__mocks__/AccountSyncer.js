// we are importing the file directly to avoid importing our own mock
import { AccountSyncer } from "$lib/../../node_modules/@dusk/w3sper/src/network/syncer/account";

import { stakeInfo } from "$lib/mock-data";

class AccountSyncerMock extends AccountSyncer {
  /**
   * @param {Network} network
   */
  constructor(network) {
    super(network);
  }

  /**
   * @param {Array<Profile>} profiles
   * @returns {Promise<AccountBalance[]>}
   */
  async balances(profiles) {
    return Array(profiles.length).fill({
      nonce: 9876n,
      value: 12_345_000_000_000n,
    });
  }

  /**
   * @param {Array<Profile>} profiles
   * @returns {Promise<StakeInfo[]>}
   */
  async stakes(profiles) {
    return Array(profiles.length).fill(stakeInfo);
  }
}

export default AccountSyncerMock;
