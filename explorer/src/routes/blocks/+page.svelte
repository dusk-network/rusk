<script>
  import { BlocksCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { onDestroy } from "svelte";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getBlocks,
    $appStore.fetchInterval
  );

  onNetworkChange((network) => {
    pollingDataStore.reset();
    pollingDataStore.start(network, $appStore.blocksListEntries);
  });

  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ blocksListEntries, network: currentNetwork } = $appStore);
</script>

<section id="blocks">
  <BlocksCard
    on:retry={() => pollingDataStore.start(currentNetwork, blocksListEntries)}
    blocks={data}
    {error}
    loading={isLoading}
  />
</section>
