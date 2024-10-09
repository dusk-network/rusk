<script>
  import { onMount } from "svelte";
  import { navigating, page } from "$app/stores";
  import { TransactionDetails } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { marketDataStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  const dataStore = createDataStore(duskAPI.getTransaction);
  const payloadStore = createDataStore(duskAPI.getTransactionDetails);

  const getTransaction = () => {
    dataStore.getData($page.url.searchParams.get("id"));
    payloadStore.getData($page.url.searchParams.get("id"));
  };

  onMount(getTransaction);

  $: if (
    $navigating &&
    $navigating.from?.route.id === $navigating.to?.route.id
  ) {
    $navigating.complete.then(getTransaction);
  }
  $: ({ data, error, isLoading } = $dataStore);
  $: ({ data: payloadData } = $payloadStore);
  $: ({ data: marketData } = $marketDataStore);
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
