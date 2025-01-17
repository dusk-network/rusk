<svelte:options immutable={true} />

<script>
  import { AppAnchor, CopyField } from "$lib/components";
  import { DOCUMENTATION_LINKS } from "$lib/constants";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { createNumberFormatter } from "$lib/dusk/number";
  import { settingsStore } from "$lib/stores";
  import { mdiCubeOutline } from "@mdi/js";
  $: ({ language } = $settingsStore);

  /** @type {bigint} */
  export let currentBlockHeight;

  const numberFormatter = createNumberFormatter(language);
</script>

<IconHeadingCard icons={[mdiCubeOutline]} heading="Block Height">
  <p>
    Store the current block height in case you want to resync from it next time
    you reset your wallet. This can significantly reduce the initial sync time.
  </p>

  <CopyField
    name="Block Height"
    displayValue={numberFormatter(currentBlockHeight)}
    rawValue={String(currentBlockHeight)}
  />

  <small>
    This can later be retrieved from Settings. Find out more in our <AppAnchor
      href={DOCUMENTATION_LINKS.RESTORE_BLOCK_HEIGHT}
      rel="noopener noreferrer"
      target="_blank">documentation</AppAnchor
    >.
  </small>
</IconHeadingCard>
