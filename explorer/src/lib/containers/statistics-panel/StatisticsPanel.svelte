<svelte:options immutable={true} />

<script>
  import {
    mdiAccountGroupOutline,
    mdiCubeOutline,
    mdiCurrencyUsd,
    mdiSwapVertical,
  } from "@mdi/js";
  import { onDestroy } from "svelte";

  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { createCompactFormatter } from "$lib/dusk/value";
  import { duskIcon } from "$lib/dusk/icons";
  import { Icon } from "$lib/dusk/components";
  import { DataGuard, StaleDataNotice, WorldMap } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import {
    createDataStore,
    createPollingDataStore,
  } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { appStore, marketDataStore } from "$lib/stores";

  import "./StatisticsPanel.css";

  const valueFormatter = createCurrencyFormatter("en", "DUSK", 0);
  const millionFormatter = createCompactFormatter("en");

  /**
   * @param { number | bigint } value
   */
  const formatter = (value) => {
    return value >= 1e6 ? millionFormatter(value) : valueFormatter(value);
  };

  const nodeLocationsStore = createDataStore(duskAPI.getNodeLocations);
  const pollingStatsDataStore = createPollingDataStore(
    duskAPI.getStats,
    $appStore.statsFetchInterval
  );

  onNetworkChange((network) => {
    nodeLocationsStore.reset();
    nodeLocationsStore.getData(network);
    pollingStatsDataStore.reset();
    pollingStatsDataStore.start(network);
  });

  onDestroy(pollingStatsDataStore.stop);

  $: ({ data: marketData } = $marketDataStore);
  $: ({ data: nodesData } = $nodeLocationsStore);
  $: ({ data: statsData } = $pollingStatsDataStore);
  $: statistics = [
    [
      {
        canBeStale: true,
        compact: true,
        data: marketData?.currentPrice.usd,
        icon: mdiCurrencyUsd,
        title: "Dusk Price",
      },
      {
        canBeStale: true,
        compact: true,
        data: marketData?.marketCap.usd,
        icon: mdiCurrencyUsd,
        title: "Total Market Cap",
      },
    ],

    [
      {
        canBeStale: false,
        compact: true,
        data: statsData?.activeStake
          ? luxToDusk(statsData?.activeStake)
          : undefined,
        icon: duskIcon,
        title: "Current Staked Amount",
      },
      {
        canBeStale: false,
        compact: true,
        data: statsData?.waitingStake
          ? luxToDusk(statsData?.waitingStake)
          : undefined,
        icon: duskIcon,
        title: "Next Epoch Staked Amount",
      },
    ],

    [
      {
        canBeStale: false,
        compact: false,
        data: statsData?.lastBlock,
        icon: mdiCubeOutline,
        title: "Last Block",
      },
      {
        canBeStale: false,
        compact: true,
        data: statsData?.txs100blocks.transfers,
        icon: mdiSwapVertical,
        title: "TX Last 100 Blocks",
      },
    ],

    [
      {
        canBeStale: false,
        compact: true,
        data: statsData?.activeProvisioners,
        icon: mdiAccountGroupOutline,
        title: "Provisioners",
      },
      {
        canBeStale: false,
        compact: true,
        data: statsData?.waitingProvisioners,
        icon: mdiAccountGroupOutline,
        title: "Next Epoch Provisioners",
      },
    ],
  ];
  $: ({ darkMode } = $appStore);
</script>

<div class="statistics-panel">
  <div class="statistics-panel__statistics">
    {#each statistics as statistic, index (index)}
      <div class="statistics-panel__statistics-column">
        {#each statistic as item (`${item.title}`)}
          <div class="statistics-panel__statistics-item">
            <div class="statistics-panel__statistics-item-value">
              <Icon path={item.icon} size="normal" />
              <DataGuard data={item.data}>
                {#if item.compact}
                  {formatter(item.data)}
                {:else}
                  {valueFormatter(item.data)}
                {/if}
              </DataGuard>
              {#if item.canBeStale}
                <StaleDataNotice />
              {/if}
            </div>
            <span class="statistics-panel__statistics-item-title"
              >{item.title}</span
            >
          </div>
        {/each}
      </div>
    {/each}
  </div>
  <div class="statistics-panel__world-map">
    <WorldMap nodes={nodesData} stroke={darkMode ? "white" : "black"} />
  </div>
</div>
