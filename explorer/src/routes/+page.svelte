<script>
  import { BlocksCard, TransactionsCard } from "$lib/components";
  import { StatisticsPanel } from "$lib/containers";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getLatestChainInfo,
    $appStore.fetchInterval
  );

  onNetworkChange(pollingDataStore.start);

  $: ({ data, error, isLoading } = $pollingDataStore);
</script>

<section class="chain-info">
  <StatisticsPanel />
</section>

<section class="tables">
  <BlocksCard
    on:retry={pollingDataStore.start}
    className="tables-layout"
    blocks={data?.blocks}
    {error}
    loading={isLoading}
  />

  <TransactionsCard
    on:retry={pollingDataStore.start}
    className="tables-layout"
    txs={data?.transactions}
    {error}
    loading={isLoading}
  />
</section>

<style lang="postcss">
  .chain-info {
    grid-template-columns: 1fr;
  }

  .tables {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    margin-top: 1.25rem;
  }

  :global(.tables-layout) {
    width: 100%;
  }

  @media (min-width: 768px) {
    .chain-info {
      display: flex;
      flex-wrap: wrap;
      gap: 1.875rem;
    }
  }

  @media (min-width: 1024px) {
    .tables {
      flex-direction: row;
      gap: 1.875rem;
    }

    :global(.tables-layout) {
      width: 50%;
    }
  }
</style>
