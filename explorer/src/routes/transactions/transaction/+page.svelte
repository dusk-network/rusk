<script>
  import { page } from "$app/stores";
  import { TransactionDetails } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { onMount } from "svelte";

  const dataStore = createDataStore(duskAPI.getTransaction);

  const getTransaction = () => {
    dataStore.getData($appStore.network, $page.url.searchParams.get("id"));
  };

  onNetworkChange(getTransaction);

  $: ({ data, error, isLoading } = $dataStore);

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
  />
</section>
