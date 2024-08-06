import { pickIn } from "lamb";

export default class MediaQueryListEvent extends Event {
  #matches;

  #media;

  /**
   * @param {string} type
   * @param {MediaQueryListEventInit} options
   */
  constructor(type, options) {
    super(type, pickIn(options, ["bubbles", "cancelable", "composed"]));

    this.#matches = options.matches;
    this.#media = options.media;
  }

  get matches() {
    return this.#matches;
  }

  get media() {
    return this.#media;
  }
}
