<script>
  import { BlocksCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onDestroy, onMount } from "svelte";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getBlocks,
    $appStore.fetchInterval
  );

  onMount(() => pollingDataStore.start($appStore.blocksListEntries));
  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ blocksListEntries, isSmallScreen } = $appStore);
</script>

<section id="blocks">
  <BlocksCard
    on:retry={() => pollingDataStore.start(blocksListEntries)}
    blocks={data}
    {error}
    loading={isLoading}
    {isSmallScreen}
  />
</section>
