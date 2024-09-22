<svelte:options immutable={true} />

<script>
  import { AppAnchor } from "$lib/components";
  import { DOCUMENTATION_LINKS } from "$lib/constants";
  import { ToggleableCard } from "$lib/containers/Cards";
  import { Icon, Textbox } from "$lib/dusk/components";
  import { createNumberFormatter } from "$lib/dusk/number";
  import { settingsStore, walletStore } from "$lib/stores";
  import { mdiAlertOutline, mdiCubeOutline } from "@mdi/js";

  const { language } = $settingsStore;
  const numberFormatter = createNumberFormatter(language);

  /** @type {number} */
  export let blockHeight = 0;

  /** @type {boolean} */
  export let isValid = false;

  /** @type {boolean} */
  export let isToggled = false;

  /** @type {number} */
  let currentNetworkBlock;

  walletStore.getCurrentBlockHeight().then((block) => {
    currentNetworkBlock = block;
  });

  $: isValid =
    !isToggled || (blockHeight >= 0 && blockHeight <= currentNetworkBlock);

  const resetBlockHeight = () => {
    blockHeight = 0;
  };
</script>

<ToggleableCard
  bind:isToggled
  iconPath={mdiCubeOutline}
  heading="Block Height"
  on:toggle={resetBlockHeight}
>
  <Textbox type="number" bind:value={blockHeight} placeholder="Block Height" />
  {#if currentNetworkBlock}
    <span class="block-height-meta"
      >Network block height: {numberFormatter(currentNetworkBlock)}</span
    >
  {/if}
</ToggleableCard>

<div class="notice">
  <Icon path={mdiAlertOutline} size="large" />
  <p>
    Syncing from a custom block height is optional. Doing so can significantly
    reduce sync times. However, setting a wrong block can lead to wrong balance
    or missing transactions. Find out more in our <AppAnchor
      href={DOCUMENTATION_LINKS.RESTORE_BLOCK_HEIGHT}
      rel="noopener noreferrer"
      target="_blank">documentation</AppAnchor
    >.
  </p>
</div>

<style>
  .block-height-meta {
    display: inline-block;
    font-size: 0.75em;
    margin-left: 1em;
    opacity: 0.5;
  }
</style>
