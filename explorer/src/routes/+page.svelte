<script>
  import { onDestroy, onMount } from "svelte";

  import {
    LatestBlocksCard,
    LatestTransactionsCard,
    WorldMap,
  } from "$lib/components";
  import { StatisticsPanel } from "$lib/containers";
  import { Card } from "$lib/dusk/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import {
    createDataStore,
    createPollingDataStore,
  } from "$lib/dusk/svelte-stores";

  const nodeLocationsStore = createDataStore(duskAPI.getNodeLocations);
  const pollingDataStore = createPollingDataStore(
    duskAPI.getLatestChainInfo,
    $appStore.fetchInterval
  );

  onMount(() => {
    nodeLocationsStore.getData();
    pollingDataStore.start($appStore.chainInfoEntries);
  });
  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ chainInfoEntries, darkMode, isSmallScreen } = $appStore);
  $: ({ data: nodesData } = $nodeLocationsStore);

  const retry = () => {
    pollingDataStore.start(chainInfoEntries);
  };
</script>

<section class="landing">
  <div class="chain-info">
    <StatisticsPanel />
  </div>

  <div class="tables">
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
  </div>

  <div class="world-map">
    <Card>
      <WorldMap nodes={nodesData} stroke={darkMode ? "white" : "black"} />
    </Card>
  </div>
</section>

<style lang="postcss">
  :global {
    .landing {
      display: flex;
      flex-direction: column;
      gap: 1.25rem;
    }

    .chain-info {
      grid-template-columns: 1fr;
      order: 1;
    }

    .tables {
      order: 2;
      display: flex;
      gap: 1.25rem;
    }

    .tables-layout {
      width: 50%;
    }

    .world-map {
      order: 3;
      flex-grow: 1;
      display: flex;
      align-items: center;
      width: 100%;
    }

    .world-map .dusk-card {
      padding: 0;
      width: 100%;
    }

    @media (min-width: 768px) {
      .chain-info {
        display: flex;
        flex-wrap: wrap;
        gap: 1.25rem;
      }
    }

    @media (max-width: 1380px) {
      .tables {
        flex-direction: column;
      }

      .tables-layout {
        width: 100%;
      }
    }

    @media (max-width: 1024px) {
      .tables {
        order: 3;
      }

      .world-map {
        order: 2;
      }
    }
  }
</style>
