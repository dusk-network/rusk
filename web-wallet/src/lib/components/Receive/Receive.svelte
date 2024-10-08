<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiContentCopy } from "@mdi/js";

  import { AppAnchorButton } from "$lib/components";
  import { Button, QrCode } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";

  /** @type {string} */
  export let address = "";

  /** @type {boolean} */
  export let hideBackButton = false;

  let offsetHeight = 0;
  let buttonHeight = 0;

  const COLUMN_COUNT = 2;
  const COLUMN_WIDTH = 16;
  const BOTTOM_PADDING = 22;

  $: qrWidth =
    offsetHeight - buttonHeight - COLUMN_COUNT * COLUMN_WIDTH - BOTTOM_PADDING;
</script>

<div class="receive" bind:offsetHeight>
  <figure class="receive__address-qr-figure">
    <QrCode value={address} className="receive__qr" width={qrWidth} />

    <figcaption class="receive__address">
      <samp>{address}</samp>
    </figcaption>
  </figure>

  <div class="receive__buttons" bind:offsetHeight={buttonHeight}>
    {#if !hideBackButton}
      <AppAnchorButton
        className="receive__button"
        icon={{ path: mdiArrowLeft }}
        href="/dashboard"
        text="Back"
        variant="tertiary"
      />
    {/if}
    <Button
      className="receive__button"
      icon={{ path: mdiContentCopy }}
      on:click={() => {
        navigator.clipboard.writeText(address);
        toast("success", "Address copied", mdiContentCopy);
      }}
      text="Copy"
    />
  </div>
</div>

<style lang="postcss">
  .receive {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    gap: var(--default-gap);
    z-index: 3;
    height: 100%;
    overflow: auto;

    &__address-qr-figure {
      display: flex;
      flex-direction: column;
      gap: var(--default-gap);
      align-items: center;
    }

    &__address,
    :global(&__qr) {
      border-radius: 1.5em;
      max-width: 100%;
      max-height: 100%;
    }

    &__address {
      line-break: anywhere;
      padding: 0.75em 1em;
      background-color: transparent;
      border: 1px solid var(--primary-color);
    }

    :global(&__qr) {
      padding: 0.625em;
      background-color: var(--background-color-alt);
    }

    :global(&__button) {
      flex: 1;
    }

    &__buttons {
      display: flex;
      flex-direction: row;
      gap: var(--default-gap);
      width: 100%;
    }
  }
</style>
