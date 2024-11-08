import { afterAll, expect } from "vitest";

const controllers = new Set();

afterAll(() => {
  controllers.forEach((controller) => {
    controller.abort();
    controllers.delete(controller);
  });
});

/** @type {PromiseWithResolvers<void>} */
let deferredRemove;

class FakeRuesScope {
  #id = "";

  #once = false;

  #name;

  /** @param {string} name */
  constructor(name) {
    this.#name = name;

    const abortController = new AbortController();

    controllers.add(abortController);

    globalThis.addEventListener(
      "transaction::removed",
      (evt) => {
        const { detail } =
          /** @type {CustomEvent<{ id: string, name: string, once: boolean }>} */ (
            evt
          );

        expect(detail.id).toBe(this.#id);
        expect(detail.name).toBe(this.#name);
        expect(detail.once).toBe(this.#once);

        deferredRemove.resolve(undefined);
      },
      { signal: abortController.signal }
    );
  }

  removed() {
    deferredRemove = Promise.withResolvers();
    return deferredRemove.promise;
  }

  /** @param {string} id */
  withId(id) {
    this.#id = id;

    return this;
  }

  get once() {
    this.#once = true;

    return this;
  }

  toJSON() {
    return {
      id: this.#id,
      name: this.#name,
      once: this.#once,
    };
  }
}

class TransactionsMock {
  #scope;

  constructor() {
    this.#scope = new FakeRuesScope("transactions");
  }

  /** @param {any} tx */
  async preverify(tx) {
    return tx;
  }

  /** @param {any} tx */
  async propagate(tx) {
    return tx;
  }

  /** @param {string} id */
  withId(id) {
    return this.#scope.withId(id);
  }

  get once() {
    return this.#scope.once;
  }
}

export default TransactionsMock;
