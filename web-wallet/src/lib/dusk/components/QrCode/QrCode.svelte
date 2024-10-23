<svelte:options immutable={true} />

<script>
  import * as QRCode from "qrcode";
  import { createEventDispatcher } from "svelte";

  import { makeClassName } from "$lib/dusk/string";
  import { AppImage } from "$lib/components";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {String} */
  export let value = "";

  /** @type {Number} */
  export let width = 200;

  /** @type {String} */
  export let qrColor = "#101";

  /** @type {String} */
  export let bgColor = "#fff";

  const dispatch = createEventDispatcher();

  /**
   * @param {string} text
   * @param {{ bgColor: string, qrColor: string, width: number }} options
   * @returns {Promise<string>}
   */
  const getDataUrl = (text, options) =>
    QRCode.toDataURL(text, {
      color: {
        dark: options.qrColor,
        light: options.bgColor,
      },
      width: options.width,
    }).catch((/** @type {String} */ error) => {
      dispatch("error", error);

      return Promise.reject(error);
    });
</script>

{#await getDataUrl(value, { bgColor, qrColor, width })}
  <div style:height={`${width}px`} style:width={`${width}px`} />
{:then url}
  <AppImage
    {...$$restProps}
    alt="Key QR code"
    className={makeClassName(["dusk-qr-code", className])}
    height={width}
    src={url}
    {width}
  />
{:catch error}
  <p>Unable to get QR code</p>
  <p>{error}</p>
{/await}

<style lang="postcss">
  :global {
    .dusk-qr-code {
      display: block;
    }
  }
</style>
