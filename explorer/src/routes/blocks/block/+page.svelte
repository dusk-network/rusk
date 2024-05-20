<script>
  import { page } from "$app/stores";
  import { BlockDetails, LatestTransactionsCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";

  const dataStore = createDataStore(duskAPI.getBlock);

  const getBlock = () => {
    dataStore.getData($appStore.network, $page.url.searchParams.get("id"));
  };

  onNetworkChange(() => {
    dataStore.reset();
    getBlock();
  });

  $: ({ data, error, isLoading } = $dataStore);
</script>

<section class="block">
  <div class="block__details">
    <BlockDetails on:retry={getBlock} {data} {error} loading={isLoading} />
  </div>
  <div class="block__transactions">
    <LatestTransactionsCard
      on:retry={getBlock}
      txs={data?.transactions.data}
      {error}
      loading={isLoading}
      isOnHomeScreen={false}
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
