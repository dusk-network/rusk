<script>
  import { TransactionsCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { onDestroy } from "svelte";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getTransactions,
    $appStore.fetchInterval
  );

  $: ({ data, error, isLoading } = $pollingDataStore);

  onNetworkChange((network) => {
    pollingDataStore.stop();
    pollingDataStore.start(network);
  });

  onDestroy(pollingDataStore.stop);
</script>

<section class="transactions">
  <TransactionsCard
    on:retry={pollingDataStore.start}
    txs={data}
    {error}
    loading={isLoading}
  />
</section>
