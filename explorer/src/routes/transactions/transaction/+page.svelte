<script>
  import { navigating, page } from "$app/stores";
  import { TransactionDetails } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import {
    createDataStore,
    createPollingDataStore,
  } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { onDestroy } from "svelte";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getTransaction,
    $appStore.fetchInterval
  );
  const payloadStore = createDataStore(duskAPI.getTransactionDetails);
  const marketStore = createDataStore(duskAPI.getMarketData);

  const getTransaction = () => {
    payloadStore.getData($appStore.network, $page.url.searchParams.get("id"));
    marketStore.getData($appStore.network);
  };

  const updateData = () => {
    pollingDataStore.reset();
    pollingDataStore.start($appStore.network, $page.url.searchParams.get("id"));
    payloadStore.reset();
    getTransaction();
  };

  onNetworkChange(updateData);

  $: if (
    $navigating &&
    $navigating.from?.route.id === $navigating.to?.route.id
  ) {
    $navigating.complete.then(updateData);
  }

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ data: payloadData } = $payloadStore);
  $: ({ data: marketData } = $marketStore);
  $: ({ network: currentNetwork } = $appStore);

  onDestroy(pollingDataStore.stop);
</script>

<section class="transaction">
  <TransactionDetails
    on:retry={() => pollingDataStore.start(currentNetwork)}
    {data}
    {error}
    loading={isLoading}
    payload={payloadData}
    market={marketData}
  />
</section>
