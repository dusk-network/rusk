import { AccountSyncer } from "$lib/vendor/w3sper.js/src/network/syncer/account";

class AccountSyncerMock extends AccountSyncer {
  /**
   * @param {import("$lib/vendor/w3sper.js/src/mod").Network} network
   * @param {Record<string, any>} [options={}]
   */
  constructor(network, options = {}) {
    super(network, options);
  }

  /**
   *
   * @param {Array<import("$lib/vendor/w3sper.js/src/mod").Profile>} profiles
   * @param {Record<string, any>} [options={}]
   * @returns {Promise<Array<{ nonce: bigint, value: bigint }>>}
   */
  // eslint-disable-next-line no-unused-vars
  async balances(profiles, options = {}) {
    return Array(profiles.length).fill({
      nonce: 9876n,
      value: 12_345_000_000_000n,
    });
  }
}

export default AccountSyncerMock;
