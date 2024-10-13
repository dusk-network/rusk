<script>
  import { onDestroy, onMount } from "svelte";

  import { LatestBlocksCard, LatestTransactionsCard } from "$lib/components";
  import { StatisticsPanel } from "$lib/containers";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getLatestChainInfo,
    $appStore.fetchInterval
  );

  onMount(() => pollingDataStore.start($appStore.chainInfoEntries));
  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ chainInfoEntries, isSmallScreen } = $appStore);

  const retry = () => {
    pollingDataStore.start(chainInfoEntries);
  };
</script>

<section class="chain-info">
  <StatisticsPanel />
</section>

<section class="tables">
  <LatestBlocksCard
    on:retry={retry}
    className="tables-layout"
    blocks={data?.blocks}
    {error}
    {isSmallScreen}
    loading={isLoading}
  />

  <LatestTransactionsCard
    on:retry={retry}
    className="tables-layout"
    txns={data?.transactions}
    {error}
    {isSmallScreen}
    loading={isLoading}
  />
</section>

<style lang="postcss">
  :global {
    .chain-info {
      grid-template-columns: 1fr;
    }

    .tables {
      display: flex;
      gap: 1.25rem;
      margin-top: 1.25rem;
    }

    .tables-layout {
      width: 50%;
    }

    @media (min-width: 48rem) {
      .chain-info {
        display: flex;
        flex-wrap: wrap;
        gap: 1.25rem;
      }
    }

    @media (max-width: 86.2rem) {
      .tables {
        flex-direction: column;
      }

      .tables-layout {
        width: 100%;
      }
    }
  }
</style>
