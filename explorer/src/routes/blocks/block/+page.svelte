<script>
  import { browser } from "$app/environment";
  import { page } from "$app/stores";
  import { BlockDetails, TransactionsCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  const dataStore = createDataStore(duskAPI.getBlock);

  const getBlock = () => {
    dataStore.getData($appStore.network, $page.url.searchParams.get("id"))
  }

  $: {
    browser && getBlock();
  }
</script>

<section class="block">
  <div class="block__details">
    <BlockDetails
      on:retry={()=>getBlock()}
      data={$dataStore.data}
      error={$dataStore.error}
      loading={$dataStore.isLoading}
    />
  </div>
  <div class="block__transactions">
    <TransactionsCard
      on:retry={()=>getBlock()}
      txs={$dataStore.data?.transactions.data}
      error={$dataStore.error}
      loading={$dataStore.isLoading}
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
