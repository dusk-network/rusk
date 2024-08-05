import { afterAll } from "vitest";

const controllers = new Set();

afterAll(() => {
  controllers.forEach((controller) => {
    controller.abort();
    controllers.delete(controller);
  });
});

/**
 * Mocks the `MediaQueryList` object and listens to the
 * "DuskMediaQueryMatchesChange" custom event.
 * Fire one manually or with the `changeMediaQueryMatches`
 * helper function to simulate media query changes.
 */
export default class MediaQueryList extends EventTarget {
  #matches;

  #media;

  /**
   * @param {string} mediaQuery
   * @param {boolean} initialMatches
   */
  constructor(mediaQuery, initialMatches) {
    super();

    this.#matches = initialMatches;
    this.#media = mediaQuery;

    const abortController = new AbortController();

    controllers.add(abortController);

    global.addEventListener("DuskMediaQueryMatchesChange", this, {
      signal: abortController.signal,
    });
  }

  get matches() {
    return this.#matches;
  }

  get media() {
    return this.#media;
  }

  /** @param {CustomEvent<{ media: string, matches: boolean }>} evt */
  handleEvent(evt) {
    const { detail, type } = evt;

    if (
      type === "DuskMediaQueryMatchesChange" &&
      detail.media === this.#media
    ) {
      this.#matches = detail.matches;

      this.dispatchEvent(
        new MediaQueryListEvent("change", {
          matches: this.#matches,
          media: this.#media,
        })
      );
    }
  }
}
