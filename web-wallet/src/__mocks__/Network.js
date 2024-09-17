import { expect } from "vitest";
import { Network } from "$lib/vendor/w3sper.js/src/network/mod";

class NetworkMock extends Network {
  /** @type {boolean} */
  #connected = false;

  /** @param {string | URL} url */
  constructor(url) {
    super(url);

    /**
     * Not ideal to have this here, but it saves us
     * the hassle of mocking the module when we need to
     * check that the correct URL is passed.
     */
    expect(url).toBe(import.meta.env.VITE_WALLET_NETWORK);
  }

  get blockHeight() {
    return Promise.resolve(123_456_789n);
  }

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
}

export default NetworkMock;
