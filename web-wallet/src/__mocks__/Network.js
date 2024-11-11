import { Network } from "$lib/vendor/w3sper.js/src/network/mod";

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

  async execute() {
    return {
      hash: "821a88f10f823b74fa3489c5acc6e31b7e2e96d7adff47137f20f4af61612415",
      nullifiers: [],
    };
  }
}

export default NetworkMock;
