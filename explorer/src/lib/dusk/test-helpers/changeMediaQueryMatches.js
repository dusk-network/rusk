/**
 * Helper to fire "DuskMediaQueryMatchesChange" custom
 * events that are listened by our `MediaQueryList` mock.
 *
 * @param {string} media
 * @param {boolean} matches
 */
export default function changeMediaQueryMatches(media, matches) {
  dispatchEvent(
    new CustomEvent("DuskMediaQueryMatchesChange", {
      detail: { matches, media },
    })
  );
}
