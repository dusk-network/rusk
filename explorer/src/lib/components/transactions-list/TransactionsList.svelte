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
  <ListItem tooltipText="The ID of the transaction">
    <svelte:fragment slot="term">Hash</svelte:fragment>
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
  <ListItem tooltipText="The date and time the transaction was created">
    <svelte:fragment slot="term">timestamp</svelte:fragment>
    <time
      datetime={data.date.toISOString()}
      class="transaction-details__list-timestamp"
      slot="definition"
    >
      <DataGuard data={data.date}>
        {getRelativeTimeString(data.date, "long")}
      </DataGuard>
    </time>
  </ListItem>

  <!-- GAS PRICE -->
  <ListItem tooltipText="The transaction gas price in lux">
    <svelte:fragment slot="term">Gas Price</svelte:fragment>
    <svelte:fragment slot="definition"
      ><DataGuard data={data.gasprice}>
        {formatter(data.gasprice)}
      </DataGuard></svelte:fragment
    >
  </ListItem>

  <!-- GAS LIMIT -->
  <ListItem tooltipText="The transaction gas limit in lux">
    <svelte:fragment slot="term">Gas Limit</svelte:fragment>
    <svelte:fragment slot="definition"
      ><DataGuard data={data.gaslimit}>
        {formatter(data.gaslimit)}
      </DataGuard></svelte:fragment
    >
  </ListItem>

  <!-- TX FEE -->
  <ListItem tooltipText="The transaction fee amount">
    <svelte:fragment slot="term">Fee</svelte:fragment>
    <svelte:fragment slot="definition"
      ><DataGuard data={data.feepaid}>
        <Badge
          variant="alt"
          text={`${formatter(luxToDusk(data.feepaid))} Dusk`}
        />
      </DataGuard></svelte:fragment
    >
  </ListItem>

  <!-- STATUS -->
  <ListItem tooltipText="The transaction status">
    <svelte:fragment slot="term">Status</svelte:fragment>
    <svelte:fragment slot="definition">
      <DataGuard data={data.success}>
        <Badge
          variant={data.success ? "success" : "error"}
          text={data.success ? "success" : "failed"}
        />
      </DataGuard>
    </svelte:fragment>
  </ListItem>

  <!-- TYPE -->
  <ListItem tooltipText="The transaction type">
    <svelte:fragment slot="term">Type</svelte:fragment>
    <svelte:fragment slot="definition"
      ><DataGuard data={data.method}>
        <Badge text={data.method} />
      </DataGuard></svelte:fragment
    >
  </ListItem>
</DetailList>
