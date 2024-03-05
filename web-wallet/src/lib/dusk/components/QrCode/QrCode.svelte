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

  /** @type {Promise<String>} */
  let dataUrlPromise;

  $: if (width || qrColor || bgColor) {
    dataUrlPromise = getDataUrl();
  }

  const getDataUrl = () =>
    QRCode.toDataURL(value, {
      color: {
        dark: qrColor,
        light: bgColor,
      },
      width,
    }).catch((/** @type {String} */ error) => {
      dispatch("error", error);

      return Promise.reject(error);
    });
</script>

{#await dataUrlPromise then url}
  <AppImage
    {...$$restProps}
    alt="Key QR code"
    className={makeClassName(["dusk-qr-code", className])}
    src={url}
  />
{:catch error}
  <p>Unable to get QR code</p>
  <p>{error}</p>
{/await}
