/** @see https://github.com/dusk-network/rusk/issues/3570 */
import "$lib/dusk/polyfill";

export const csr = true;
export const prerender = true;
export const ssr = false;
export const trailingSlash = "always";

/** @type {import("./$types").LayoutLoad} */
export async function load() {
  let needsStreamsPolyfill =
    !globalThis.ReadableStream ||
    !globalThis.TransformStream ||
    !globalThis.ReadableStream.prototype?.pipeTo ||
    !globalThis.ReadableStream.prototype?.[Symbol.asyncIterator];

  if (!needsStreamsPolyfill) {
    try {
      // Some versions of Safari supports streams but not the BYOB mode
      new ReadableStream({ type: "bytes" }).getReader({ mode: "byob" });
    } catch {
      // eslint-disable-next-line no-console
      console.warn("Native BYOB support missing, forcing polyfill");
      needsStreamsPolyfill = true;
    }
  }

  if (needsStreamsPolyfill) {
    await import("web-streams-polyfill/polyfill");

    // eslint-disable-next-line no-console
    console.info(
      "Web Streams polyfilled with `web-streams-polyfill` via layout load"
    );
  }

  return {};
}
