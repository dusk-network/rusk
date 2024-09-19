<svelte:options immutable={true} />

<script>
  import { AppAnchor, CopyField } from "$lib/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { createNumberFormatter } from "$lib/dusk/number";
  import { settingsStore, walletStore } from "$lib/stores";
  import { mdiCubeOutline } from "@mdi/js";
  $: ({ language } = $settingsStore);

  /** @type {number} */
  let currentBlock;

  walletStore.getCurrentBlockHeight().then((block) => {
    currentBlock = block;
  });

  const numberFormatter = createNumberFormatter(language);
</script>

<IconHeadingCard icons={[mdiCubeOutline]} heading="Block Height">
  <p>
    Store the current block height in case you want to resync from it next time
    you reset your wallet. This can significantly reduce the initial sync time.
  </p>

  <CopyField
    name="Block Height"
    displayValue={currentBlock ? numberFormatter(currentBlock) : "Loading..."}
    rawValue={String(currentBlock)}
    disabled={!currentBlock}
  />

  <small>
    This can later be retrieved from Settings. Find out more in our <AppAnchor
      href="#"
      rel="noopener noreferrer"
      target="_blank">documentation</AppAnchor
    >.
  </small>
</IconHeadingCard>
