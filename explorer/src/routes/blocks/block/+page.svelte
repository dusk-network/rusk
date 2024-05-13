<script>
  import { page } from "$app/stores";
  import { BlockDetails, TransactionsCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { onMount } from "svelte";

  const dataStore = createDataStore(duskAPI.getBlock);

  const getBlock = () => {
    dataStore.getData($appStore.network, $page.url.searchParams.get("id"));
  };

  onNetworkChange(getBlock);

  $: ({ data, error, isLoading } = $dataStore);

  onMount(() => {
    getBlock();
  });
</script>

<section class="block">
  <div class="block__details">
    <BlockDetails on:retry={getBlock} {data} {error} loading={isLoading} />
  </div>
  <div class="block__transactions">
    <TransactionsCard
      on:retry={getBlock}
      txs={data?.transactions.data}
      {error}
      loading={isLoading}
    />
  </div>
</section>

<style lang="postcss">
  .block {
    display: flex;
    flex-direction: column;
    row-gap: 1.875em;
  }

  @media (max-width: 768px) {
    .block {
      row-gap: 1.25em;
    }
  }
</style>
