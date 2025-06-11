<script>
  import { onMount } from "svelte";
  import { navigating, page } from "$app/stores";
  import { TransactionDetails } from "$lib/components/";
  import { Banner } from "$lib/dusk/components/";
  import { duskAPI } from "$lib/services";
  import { marketDataStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  const dataStore = createDataStore(duskAPI.getTransaction);
  const getTransaction = async () => {
    const id = $page.url.searchParams.get("id");
    await dataStore.getData(id);
  };

  onMount(getTransaction);

  $: if (
    $navigating &&
    $navigating.from?.route.id === $navigating.to?.route.id
  ) {
    $navigating.complete.then(getTransaction);
  }
  $: ({ data, error, isLoading } = $dataStore);
  $: ({ data: marketData } = $marketDataStore);
</script>

<section class="transaction">
  {#if typeof data === "string"}
    <Banner title="This transaction is being processed" variant="info">
      {data}
    </Banner>
  {:else}
    <TransactionDetails
      on:retry={getTransaction}
      {data}
      {error}
      loading={isLoading}
      market={marketData}
    />
  {/if}
</section>
