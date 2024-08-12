import { describe, expect, it } from "vitest";

import { changeMediaQueryMatches } from "..";

describe("changeMediaQueryMatches", () => {
  it('should dispatch "DuskMediaQueryMatchesChange" custom events', () => {
    const media = "(max-width: 1024px)";
    const matches = true;

    /** @param {Event} evt */
    const handler = (evt) => {
      expect(evt).toBeInstanceOf(CustomEvent);
      expect(evt.type).toBe("DuskMediaQueryMatchesChange");

      // @ts-ignore see https://github.com/Microsoft/TypeScript/issues/28357
      expect(evt.detail).toStrictEqual({ matches, media });
    };

    global.addEventListener("DuskMediaQueryMatchesChange", handler);

    changeMediaQueryMatches(media, matches);

    global.removeEventListener("DuskMediaQueryMatchesChange", handler);

    expect.assertions(3);
  });
});
