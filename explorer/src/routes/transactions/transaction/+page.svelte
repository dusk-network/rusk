<script>
  import { page } from "$app/stores";
  import { TransactionDetails } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { onMount } from "svelte";

  const dataStore = createDataStore(duskAPI.getTransaction);
  const payloadStore = createDataStore(duskAPI.getTransactionDetails);
  const marketStore = createDataStore(duskAPI.getMarketData);

  const getTransaction = () => {
    dataStore.getData($appStore.network, $page.url.searchParams.get("id"));
    payloadStore.getData($appStore.network, $page.url.searchParams.get("id"));
    marketStore.getData($appStore.network);
  };

  onNetworkChange(getTransaction);

  $: ({ data, error, isLoading } = $dataStore);
  $: ({ data: payloadData } = $payloadStore);
  $: ({ data: marketData } = $marketStore);

  onMount(() => {
    getTransaction();
  });
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
