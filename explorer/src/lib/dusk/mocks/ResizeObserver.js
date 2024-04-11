export default class ResizeObserver {
  /** @param {Function} callback */
  constructor(callback) {
    this.#callback = callback;
  }

  #callback;

  disconnect() {}

  observe() {
    this.#callback();
  }

  unobserve() {}
}
