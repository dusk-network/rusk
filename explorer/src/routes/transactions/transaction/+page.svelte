<script>
  import { browser } from "$app/environment";
  import { page } from "$app/stores";
  import { TransactionDetails } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  const dataStore = createDataStore(duskAPI.getTransaction);

  const getTransaction = () => {
    dataStore.getData($appStore.network, $page.url.searchParams.get("id"))
  }

  $: {
    browser && getTransaction();
  }
</script>

<section class="transaction">
  <TransactionDetails
    on:retry={()=>getTransaction()}
    data={$dataStore.data}
    error={$dataStore.error}
    loading={$dataStore.isLoading}
  />
</section>
