<svelte:options immutable={true} />

<script>
  import { createEventDispatcher } from "svelte";
  import { mdiArrowLeft, mdiContentCopy } from "@mdi/js";

  import { Button, QrCode } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";

  /** @type {string} */
  export let address = "";

  /** @type {boolean} */
  export let hideBackButton = false;

  let offsetHeight = 0;
  let receiveAddressHeight = 0;
  let buttonHeight = 0;

  const COLUMN_COUNT = 2;
  const COLUMN_WIDTH = 16;
  const BOTTOM_PADDING = 22;

  const dispatch = createEventDispatcher();

  $: qrWidth =
    offsetHeight -
    receiveAddressHeight -
    buttonHeight -
    COLUMN_COUNT * COLUMN_WIDTH -
    BOTTOM_PADDING;
</script>

<div class="receive" bind:offsetHeight>
  <figure class="receive__address-qr-figure">
    <QrCode value={address} className="receive__qr" width={qrWidth} />

    <figcaption
      class="receive__address"
      bind:offsetHeight={receiveAddressHeight}
    >
      <samp>{address}</samp>
    </figcaption>
  </figure>

  <div class="receive__buttons" bind:offsetHeight={buttonHeight}>
    {#if !hideBackButton}
      <Button
        className="flex-1"
        icon={{ path: mdiArrowLeft }}
        on:click={() => {
          dispatch("operationChange", "");
        }}
        text="Back"
        variant="tertiary"
      />
    {/if}
    <Button
      className="flex-1"
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
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    gap: var(--default-gap);
    background-color: var(--background-color);
    top: 0;
    left: 0;
    z-index: 3;
    height: 100%;

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

    &__buttons {
      display: flex;
      flex-direction: row;
      gap: var(--default-gap);
      width: 100%;
    }
  }
</style>
