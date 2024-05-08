<svelte:options immutable={true} />

<script>
  import { browser } from "$app/environment";
  import {
    mdiAccountGroupOutline,
    mdiCubeOutline,
    mdiCurrencyUsd,
    mdiSwapVertical,
  } from "@mdi/js";
  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { createCompactFormatter } from "$lib/dusk/value";
  import { duskIcon } from "$lib/dusk/icons";
  import { Icon } from "$lib/dusk/components";
  import { DataGuard, WorldMap } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore, createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import "./StatisticsPanel.css";

  const valueFormatter = createCurrencyFormatter("en", "DUSK", 0);
  const millionFormatter = createCompactFormatter("en");

  /**
   * @param { number | bigint } value
   */
  const formatter = (value) => {
    return value >= 1e6 ? millionFormatter(value) : valueFormatter(value);
  };

  const dataStore = createDataStore(duskAPI.getNodeLocations);
  const pollingMarketDataStore = createPollingDataStore(
    duskAPI.getMarketData,
    $appStore.fetchInterval
  );

  const pollingStatsDataStore = createPollingDataStore(
    duskAPI.getStats,
    $appStore.fetchInterval
  );

  onNetworkChange(pollingMarketDataStore.start);
  onNetworkChange(pollingStatsDataStore.start);

  $: {
    browser && dataStore.getData($appStore.network);
  }

  $: statistics = [
    [
      {
        data: $pollingMarketDataStore.data?.currentPrice.usd,
        icon: mdiCurrencyUsd,
        title: "Dusk Price",
      },
      {
        data: $pollingMarketDataStore.data?.marketCap.usd,
        icon: mdiCurrencyUsd,
        title: "Total Market Cap",
      },
    ],

    [
      {
        data: $pollingStatsDataStore.data?.activeStake
          ? luxToDusk($pollingStatsDataStore.data.activeStake)
          : undefined,
        icon: duskIcon,
        title: "Current Staked Amount",
      },
      {
        data: $pollingStatsDataStore.data?.waitingStake
          ? luxToDusk($pollingStatsDataStore.data.waitingStake)
          : undefined,
        icon: duskIcon,
        title: "Next Epoch Staked Amount",
      },
    ],

    [
      {
        data: $pollingStatsDataStore.data?.lastBlock,
        icon: mdiCubeOutline,
        title: "Last Block",
      },
      {
        data: $pollingStatsDataStore.data?.txs100blocks.transfers,
        icon: mdiSwapVertical,
        title: "TX Last 100 Blocks",
      },
    ],

    [
      {
        data: $pollingStatsDataStore.data?.activeProvisioners,
        icon: mdiAccountGroupOutline,
        title: "Provisioners",
      },
      {
        data: $pollingStatsDataStore.data?.waitingProvisioners,
        icon: mdiAccountGroupOutline,
        title: "Next Epoch Provisioners",
      },
    ],
  ];
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
                {formatter(item.data)}
              </DataGuard>
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
      <WorldMap nodes={$dataStore.data} />
  </div>
</div>
