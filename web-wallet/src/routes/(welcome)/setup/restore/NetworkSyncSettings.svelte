<svelte:options immutable={true} />

<script>
  import { mdiCubeOutline } from "@mdi/js";

  import { createNumberFormatter } from "$lib/dusk/number";
  import { makeClassName } from "$lib/dusk/string";

  import { AppAnchor, Banner, BigIntInput } from "$lib/components";
  import { DOCUMENTATION_LINKS } from "$lib/constants";
  import { ToggleableCard } from "$lib/containers/Cards";

  import { networkStore, settingsStore } from "$lib/stores";

  /** @type {bigint} */
  export let blockHeight = 0n;

  /** @type {boolean} */
  export let isValid = false;

  /** @type {boolean} */
  export let isToggled = false;

  /** @type {bigint} */
  let currentNetworkBlock;

  /** @type {boolean} */
  let errorFetchingCurrentNetworkBlock;

  const { language } = $settingsStore;
  const numberFormatter = createNumberFormatter(language);

  const resetBlockHeight = () => {
    blockHeight = 0n;
  };

  networkStore
    .getCurrentBlockHeight()
    .then((block) => {
      currentNetworkBlock = block;
    })
    .catch(() => {
      errorFetchingCurrentNetworkBlock = true;
    });

  $: {
    isValid =
      !isToggled || (blockHeight >= 0 && blockHeight <= currentNetworkBlock);
  }

  $: inputClasses = makeClassName({
    "block-height-input": true,
    "block-height-input--invalid": !isValid,
  });
</script>

<ToggleableCard
  bind:isToggled
  iconPath={mdiCubeOutline}
  heading="Block Height"
  on:toggle={resetBlockHeight}
>
  <BigIntInput className={inputClasses} bind:value={blockHeight} />

  {#if currentNetworkBlock}
    <span class="block-height-meta"
      >Network block height: {numberFormatter(currentNetworkBlock)}</span
    >
  {/if}

  {#if errorFetchingCurrentNetworkBlock}
    <Banner
      title="Unable to fetch current network block height."
      variant="error"
    >
      <p>Please try again later.</p>
    </Banner>
  {/if}
</ToggleableCard>

<Banner title="Syncing from a custom block height is optional." variant="info">
  <p>
    Doing so can significantly reduce sync times. However, setting a wrong block
    can lead to wrong balance or missing transactions. Find out more in our <AppAnchor
      href={DOCUMENTATION_LINKS.RESTORE_BLOCK_HEIGHT}
      rel="noopener noreferrer"
      target="_blank">documentation</AppAnchor
    >.
  </p>
</Banner>

<style lang="postcss">
  :global {
    .block-height-meta {
      display: inline-block;
      font-size: 0.75em;
      margin-left: 1em;
      opacity: 0.5;
    }

    .block-height-input--invalid {
      color: var(--error-color);
    }
  }
</style>
