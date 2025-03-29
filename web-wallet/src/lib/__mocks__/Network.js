// we are importing the file directly to avoid importing our own mock
import { Network } from "$lib/../../node_modules/@dusk/w3sper/src/network/mod";

class NetworkMock extends Network {
  /** @type {boolean} */
  #connected = false;

  /** @param {string | URL} url */
  constructor(url) {
    super(url);
  }

  get blockHeight() {
    return Promise.resolve(123_456_789n);
  }

  // @ts-ignore
  node = {
    info: Promise.resolve({
      chain: function toString() {
        return "localnet";
      },
    }),
  };

  get connected() {
    return this.#connected;
  }

  async connect() {
    this.#connected = true;

    return this;
  }

  async disconnect() {
    this.#connected = false;
  }

  /**
   * For our current tests we always return a `ShieldedTransferResult`
   * @param {import("@dusk/w3sper").BasicTransfer} tx
   * @returns {Promise<import("@dusk/w3sper").ShieldedTransferResult>}
   */
  // eslint-disable-next-line no-unused-vars
  async execute(tx) {
    return Object.freeze({
      buffer: new Uint8Array(),
      hash: "821a88f10f823b74fa3489c5acc6e31b7e2e96d7adff47137f20f4af61612415",
      nullifiers: [],
    });
  }
}

export default NetworkMock;
