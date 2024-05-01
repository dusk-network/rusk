<svelte:options immutable={true} />

<script>
  import {
    mdiAccountGroupOutline,
    mdiCubeOutline,
    mdiCurrencyUsd,
    mdiSwapVertical,
  } from "@mdi/js";
  import { createCurrencyFormatter } from "$lib/dusk/currency";
  import { createCompactFormatter } from "$lib/dusk/value";
  import { duskIcon } from "$lib/dusk/icons";
  import { Icon } from "$lib/dusk/components";
  import { WorldMap } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import "./StatisticsPanel.css";

  const valueFormatter = createCurrencyFormatter("en", "DUSK", 0);
  const millionFormatter = createCompactFormatter("en");

  /**
   * @param { number | bigint } value
   */
  const formatter = (value) => {
    return value >= 1e6 ? millionFormatter(value) : valueFormatter(value);
  };

  const statistics = [
    [
      {
        data: 0.65,
        icon: mdiCurrencyUsd,
        title: "Dusk Price",
      },
      {
        data: 123446789,
        icon: mdiCurrencyUsd,
        title: "Total Market Cap",
      },
    ],

    [
      {
        data: undefined,
        icon: duskIcon,
        title: "Epoch Staked Amount",
      },
      {
        data: undefined,
        icon: duskIcon,
        title: "Next Epoch Staked Amount",
      },
    ],

    [
      {
        data: 123654,
        icon: mdiCubeOutline,
        title: "Last Block",
      },
      {
        data: undefined,
        icon: mdiSwapVertical,
        title: "TX Last 100 Blocks",
      },
    ],

    [
      {
        data: 4500,
        icon: mdiAccountGroupOutline,
        title: "Provisioners",
      },
      {
        data: undefined,
        icon: mdiAccountGroupOutline,
        title: "Next Epoch Provisioners",
      },
    ],
  ];

  const nodeLocations = duskAPI.getNodeLocations($appStore.network);
</script>

<div class="statistics-panel">
  <div class="statistics-panel__statistics">
    {#each statistics as statistic, index (index)}
      <div class="statistics-panel__statistics-column">
        {#each statistic as item (`${item.title}`)}
          <div class="statistics-panel__statistics-item">
            <div class="statistics-panel__statistics-item-value">
              <Icon path={item.icon} size="normal" />
              {#if item.data}
                <span>{formatter(item.data)}</span>
              {:else}
                <span>- - -</span>
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
    {#await nodeLocations then nodes}
      <WorldMap {nodes} />
    {/await}
  </div>
</div>
