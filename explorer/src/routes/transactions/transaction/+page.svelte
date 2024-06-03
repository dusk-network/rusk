<script>
  import { navigating, page } from "$app/stores";
  import { TransactionDetails } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";

  const dataStore = createDataStore(duskAPI.getTransaction);
  const payloadStore = createDataStore(duskAPI.getTransactionDetails);
  const marketStore = createDataStore(duskAPI.getMarketData);

  const getTransaction = () => {
    dataStore.getData($appStore.network, $page.url.searchParams.get("id"));
    payloadStore.getData($appStore.network, $page.url.searchParams.get("id"));
    marketStore.getData($appStore.network);
  };

  const updateData = () => {
    dataStore.reset();
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

  $: ({ data, error, isLoading } = $dataStore);
  $: ({ data: payloadData } = $payloadStore);
  $: ({ data: marketData } = $marketStore);
</script>

<section class="transaction">
  <TransactionDetails
    on:retry={getTransaction}
    {data}
    {error}
    loading={isLoading}
    payload={payloadData}
    market={marketData}
  />
</section>
