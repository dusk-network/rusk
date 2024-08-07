<svelte:options immutable={true} />

<script>
  import { AppAnchor, DataGuard, DetailList, ListItem } from "$lib/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    calculateAdaptiveCharCount,
    getRelativeTimeString,
    middleEllipsis,
  } from "$lib/dusk/string";
  import { Badge } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { onMount } from "svelte";

  import "./TransactionsList.css";

  /** @type {Transaction}*/
  export let data;

  /** @type {"compact" | "full"} */
  export let mode;

  /** @type {Boolean} */
  export let displayTooltips = false;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const formatter = createValueFormatter("en");

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });
</script>

<DetailList>
  <!-- TRANSACTION ID -->
  <ListItem tooltipText={displayTooltips ? "The ID of the transaction" : ""}>
    <svelte:fragment slot="term">ID</svelte:fragment>
    <svelte:fragment slot="definition">
      <AppAnchor
        className="transaction-details__list-link"
        href={`/transactions/transaction?id=${data.txid}`}
        >{middleEllipsis(
          data.txid,
          calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 30)
        )}</AppAnchor
      >
    </svelte:fragment>
  </ListItem>

  <!-- TIMESTAMP -->
  <ListItem
    tooltipText={displayTooltips
      ? "Time elapsed since the transaction was created"
      : ""}
  >
    <svelte:fragment slot="term">relative time</svelte:fragment>
    <time
      datetime={data.date.toISOString()}
      class="transaction-details__list-timestamp"
      slot="definition"
    >
      {getRelativeTimeString(data.date, "long")}
    </time>
  </ListItem>

  {#if mode === "full"}
    <!-- GAS PRICE -->
    <ListItem
      tooltipText={displayTooltips ? "The transaction gas price in lux" : ""}
    >
      <svelte:fragment slot="term">Gas Price</svelte:fragment>
      <svelte:fragment slot="definition">
        {formatter(data.gasprice)}
      </svelte:fragment>
    </ListItem>

    <!-- GAS LIMIT -->
    <ListItem
      tooltipText={displayTooltips ? "The transaction gas limit in lux" : ""}
    >
      <svelte:fragment slot="term">Gas Limit</svelte:fragment>
      <svelte:fragment slot="definition">
        {formatter(data.gaslimit)}
      </svelte:fragment>
    </ListItem>
  {/if}

  <!-- TX FEE -->
  <ListItem tooltipText={displayTooltips ? "The transaction fee amount" : ""}>
    <svelte:fragment slot="term">Fee</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(luxToDusk(data.feepaid))} DUSK
    </svelte:fragment>
  </ListItem>

  <!-- STATUS -->
  <ListItem tooltipText={displayTooltips ? "The transaction status" : ""}>
    <svelte:fragment slot="term">Status</svelte:fragment>
    <svelte:fragment slot="definition">
      <DataGuard data={data.success}>
        <Badge
          className="explorer-badge"
          variant={data.success ? "success" : "error"}
          text={data.success ? "success" : "failed"}
        />
      </DataGuard>
    </svelte:fragment>
  </ListItem>

  <!-- TYPE -->
  <ListItem tooltipText={displayTooltips ? "The transaction type" : ""}>
    <svelte:fragment slot="term">Type</svelte:fragment>
    <svelte:fragment slot="definition"
      ><DataGuard data={data.method}>
        <Badge className="explorer-badge" text={data.method} />
      </DataGuard></svelte:fragment
    >
  </ListItem>
</DetailList>
