<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiContentCopy } from "@mdi/js";
  import { onMount } from "svelte";

  import { AppAnchorButton } from "$lib/components";
  import { Button, QrCode } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";

  /** @type {string} */
  export let address = "";

  /** @type {boolean} */
  export let hideBackButton = false;

  /** @type {HTMLElement} */
  let figureElement;

  let qrWidth = 0;

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      qrWidth = entries[0].contentRect.width;
    });

    resizeObserver.observe(figureElement);

    return () => resizeObserver.disconnect();
  });
</script>

<div class="receive">
  <figure class="receive__address-qr-figure" bind:this={figureElement}>
    <QrCode value={address} className="receive__qr" width={qrWidth} />

    <figcaption class="receive__address">
      <samp>{address}</samp>
    </figcaption>
  </figure>

  <div class="receive__buttons">
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

    &__address-qr-figure {
      display: flex;
      flex-direction: column;
      gap: var(--default-gap);
      align-items: center;
    }

    &__address,
    :global(&__qr) {
      border-radius: 1.5em;
      width: 100%;
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
