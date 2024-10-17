import { Gas } from "$lib/vendor/w3sper.js/src/network/gas";

/**
 * @typedef {Uint8Array | import("$lib/vendor/w3sper.js/src/profile").Profile["address"]} Identifier
 */

class TransactionBuilderMock {
  #amount = 0n;
  #bookkeeper;

  /** @type {Identifier} */
  #from = new Uint8Array();

  #gas;

  #obfuscated = false;

  /** @type {Identifier} */
  #to = new Uint8Array();

  /** @param {import("$lib/vendor/w3sper.js/src/bookkeeper").Bookkeeper} bookkeeper */
  constructor(bookkeeper) {
    this.#bookkeeper = bookkeeper;
    this.#gas = new Gas();
  }

  /** @param {bigint} value */
  amount(value) {
    this.#amount = value;

    return this;
  }

  /** @param {Identifier} identifier */
  from(identifier) {
    this.#from = identifier;

    return this;
  }

  /** @param {Gas} value */
  gas(value) {
    this.#gas = value;

    return this;
  }

  obfuscated() {
    this.#obfuscated = true;

    return this;
  }

  /** @param {Identifier} identifier */
  to(identifier) {
    this.#to = identifier;

    return this;
  }

  toJSON() {
    return {
      amount: this.#amount,
      from: this.#from.toString(),
      gas: this.#gas,
      obfuscated: this.#obfuscated,
      to: this.#to,
    };
  }
}

export default TransactionBuilderMock;
