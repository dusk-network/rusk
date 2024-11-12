<svelte:options immutable={true} />

<script>
  import {
    mdiAccountGroupOutline,
    mdiCubeOutline,
    mdiCurrencyUsd,
    mdiMapMarker,
    mdiSwapVertical,
    mdiTransitConnectionVariant,
  } from "@mdi/js";
  import { onDestroy, onMount } from "svelte";

  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { createCompactFormatter } from "$lib/dusk/value";
  import { duskIcon } from "$lib/dusk/icons";
  import { Icon } from "$lib/dusk/components";
  import { DataGuard, StaleDataNotice } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { appStore, marketDataStore, nodeLocationStore } from "$lib/stores";

  import "./StatisticsPanel.css";

  const valueFormatter = createCurrencyFormatter("en", "DUSK", 0);
  const millionFormatter = createCompactFormatter("en");

  /**
   * @param { number | bigint } value
   */
  const formatter = (value) => {
    return value >= 1e6 ? millionFormatter(value) : valueFormatter(value);
  };

  const pollingStatsDataStore = createPollingDataStore(
    duskAPI.getStats,
    $appStore.statsFetchInterval
  );

  onMount(() => {
    pollingStatsDataStore.start();
  });

  onDestroy(pollingStatsDataStore.stop);

  $: ({ data: marketData } = $marketDataStore);
  $: ({ data: statsData } = $pollingStatsDataStore);
  $: ({ data: nodesData } = $nodeLocationStore);
  $: statistics = [
    [
      {
        approximate: false,
        attributes: null,
        canBeStale: true,
        compact: true,
        data: marketData?.currentPrice.usd,
        icon: mdiCurrencyUsd,
        title: "Dusk Price",
      },
      {
        approximate: false,
        attributes: null,
        canBeStale: true,
        compact: true,
        data: marketData?.marketCap.usd,
        icon: mdiCurrencyUsd,
        title: "Total Market Cap",
      },
    ],

    [
      {
        approximate: !!statsData?.activeStake,
        attributes: {
          "data-tooltip-id": "main-tooltip",
          "data-tooltip-place": "top",
          "data-tooltip-text": luxToDusk(statsData?.activeStake)
            ? `${luxToDusk(statsData?.activeStake)} DUSK`
            : "--- DUSK",
          "data-tooltip-type": "info",
        },
        canBeStale: false,
        compact: true,
        data: statsData?.activeStake
          ? luxToDusk(statsData?.activeStake)
          : undefined,
        icon: duskIcon,
        title: "Current Stake",
      },
      {
        approximate: !!statsData?.waitingStake,
        attributes: {
          "data-tooltip-id": "main-tooltip",
          "data-tooltip-place": "top",
          "data-tooltip-text": luxToDusk(statsData?.waitingStake)
            ? `${luxToDusk(statsData?.waitingStake)} DUSK`
            : "--- DUSK",
          "data-tooltip-type": "info",
        },
        canBeStale: false,
        compact: true,
        data: statsData?.waitingStake
          ? luxToDusk(statsData?.waitingStake)
          : undefined,
        icon: duskIcon,
        title: "Pending Stake",
      },
    ],

    [
      {
        approximate: false,
        attributes: null,
        canBeStale: false,
        compact: false,
        data: statsData?.lastBlock,
        icon: mdiCubeOutline,
        title: "Last Block",
      },
      {
        approximate: false,
        attributes: null,
        canBeStale: false,
        compact: true,
        data: statsData?.txs100blocks.transfers,
        icon: mdiSwapVertical,
        title: "TX Last 100 Blocks",
      },
    ],

    [
      {
        approximate: false,
        attributes: null,
        canBeStale: false,
        compact: true,
        data: statsData?.activeProvisioners,
        icon: mdiAccountGroupOutline,
        title: "Active Provisioners",
      },
      {
        approximate: false,
        attributes: null,
        canBeStale: false,
        compact: true,
        data: statsData?.waitingProvisioners,
        icon: mdiAccountGroupOutline,
        title: "Pending Provisioners",
      },
    ],

    [
      {
        approximate: false,
        attributes: null,
        canBeStale: false,
        compact: true,
        data: nodesData?.length ? nodesData[0].count : null,
        icon: mdiTransitConnectionVariant,
        title: "Largest Node Cluster",
      },
      {
        approximate: false,
        attributes: null,
        canBeStale: false,
        compact: null,
        data: nodesData?.length
          ? `${nodesData[0].city}, ${nodesData[0].countryCode}`
          : null,
        icon: mdiMapMarker,
        title: "Largest Cluster Location",
      },
    ],
  ];
</script>

<div class="statistics-panel">
  <div class="statistics-panel__statistics">
    {#each statistics as statistic, index (index)}
      <div class="statistics-panel__statistics-column">
        {#each statistic as item (`${item.title}`)}
          <div class="statistics-panel__statistics-item" {...item.attributes}>
            <div class="statistics-panel__statistics-item-value-container">
              <Icon path={item.icon} size="normal" />
              <div
                class="statistics-panel__statistics-item-value"
                class:approximate={item.approximate}
              >
                <DataGuard data={item.data}>
                  {#if item.compact}
                    {formatter(item.data)}
                  {:else if item.compact === null}
                    {item.data}
                  {:else}
                    {valueFormatter(item.data)}
                  {/if}
                </DataGuard>
                {#if item.canBeStale}
                  <StaleDataNotice />
                {/if}
              </div>
            </div>
            <span class="statistics-panel__statistics-item-title"
              >{item.title}</span
            >
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>
