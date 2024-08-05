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

  onNetworkChange((network) => {
    pollingDataStore.reset();
    pollingDataStore.start(network, $appStore.transactionsListEntries);
  });

  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ network: currentNetwork, transactionsListEntries } = $appStore);
</script>

<section id="transactions">
  <TransactionsCard
    on:retry={() =>
      pollingDataStore.start(currentNetwork, transactionsListEntries)}
    txns={data}
    {error}
    loading={isLoading}
    {appStore}
  />
</section>
