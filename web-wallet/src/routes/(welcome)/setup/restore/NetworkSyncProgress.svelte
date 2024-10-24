<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { mdiCubeOutline } from "@mdi/js";

  import { ErrorAlert } from "$lib/dusk/components";

  import { SyncBar } from "$lib/components";
  import IconHeadingCard from "$lib/containers/Cards/IconHeadingCard.svelte";

  import { walletStore } from "$lib/stores";

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
  {#if !syncStarted || (syncStatus.isInProgress && !syncStatus.progress)}
    <span>Syncing...</span>
  {:else if syncStatus.isInProgress}
    <span>Syncing... <b>{syncStatus.progress * 100}%</b></span>
    <SyncBar
      from={syncStatus.from}
      last={syncStatus.last}
      progress={syncStatus.progress}
    />
  {:else if syncStatus.error}
    <ErrorAlert error={syncStatus.error} summary="Sync failed" />
  {:else}
    <span>Sync completed!</span>
  {/if}
</IconHeadingCard>
