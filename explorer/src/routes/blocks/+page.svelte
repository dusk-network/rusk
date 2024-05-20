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

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ network: currentNetwork } = $appStore);

  onNetworkChange((network) => {
    pollingDataStore.reset();
    pollingDataStore.start(network);
  });

  onDestroy(pollingDataStore.stop);
</script>

<section id="blocks">
  <BlocksCard
    on:retry={() => pollingDataStore.start(currentNetwork)}
    blocks={data}
    {error}
    loading={isLoading}
  />
</section>
