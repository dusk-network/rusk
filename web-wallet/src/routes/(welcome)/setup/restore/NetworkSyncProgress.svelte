<svelte:options immutable={true} />

<script>
  import IconHeadingCard from "$lib/containers/Cards/IconHeadingCard.svelte";
  import { ErrorAlert, ProgressBar } from "$lib/dusk/components";
  import { walletStore } from "$lib/stores";
  import { mdiCubeOutline } from "@mdi/js";
  import { onDestroy } from "svelte";

  $: ({ syncStatus } = $walletStore);

  /** @type {boolean} */
  export let isValid = false;

  let syncStarted = false;
  $: if (!syncStarted && syncStatus.isInProgress) {
    syncStarted = true;
  }

  $: isValid = syncStarted && !syncStatus.isInProgress && !syncStatus.error;

  onDestroy(() => {
    walletStore.abortSync();
  });
</script>

<IconHeadingCard icons={[mdiCubeOutline]} heading="Network Sync">
  {#if !syncStarted || (syncStatus.isInProgress && (!syncStatus.current || !syncStatus.last))}
    <span>Syncing...</span>
  {:else if syncStatus.isInProgress}
    <span>
      Syncing: <b
        >{syncStatus.current.toLocaleString()}/{syncStatus.last.toLocaleString()}</b
      >
    </span>
    <ProgressBar
      currentPercentage={(syncStatus.current / syncStatus.last) * 100}
    />
  {:else if syncStatus.error}
    <ErrorAlert error={syncStatus.error} summary="Sync failed" />
  {:else}
    <span>Sync completed!</span>
  {/if}
</IconHeadingCard>
