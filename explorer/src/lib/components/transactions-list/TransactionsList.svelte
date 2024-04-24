<svelte:options immutable={true} />

<script>
  import { DetailList, ListItem } from "$lib/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    calculateAdaptiveCharCount,
    getRelativeTimeString,
    middleEllipsis,
  } from "$lib/dusk/string";
  import { Badge } from "$lib/dusk/components";
  import { onMount } from "svelte";

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
      {middleEllipsis(
        data.txid,
        calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 30)
      )}
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
      {getRelativeTimeString(data.date, "long")}
    </time>
  </ListItem>

  <!-- GAS PRICE -->
  <ListItem tooltipText="The ID of the transaction">
    <svelte:fragment slot="term">Gas Price</svelte:fragment>
    <svelte:fragment slot="definition"
      >{formatter(data.gasprice)}</svelte:fragment
    >
  </ListItem>

  <!-- GAS LIMIT -->
  <ListItem tooltipText="The ID of the transaction">
    <svelte:fragment slot="term">Gas Limit</svelte:fragment>
    <svelte:fragment slot="definition"
      >{formatter(data.gaslimit)}</svelte:fragment
    >
  </ListItem>

  <!-- TX FEE -->
  <ListItem tooltipText="The ID of the transaction">
    <svelte:fragment slot="term">Fee</svelte:fragment>
    <svelte:fragment slot="definition"
      >{formatter(data.feepaid)}</svelte:fragment
    >
  </ListItem>

  <!-- STATUS -->
  <ListItem tooltipText="The ID of the transaction">
    <svelte:fragment slot="term">Status</svelte:fragment>
    <svelte:fragment slot="definition">
      <Badge
        variant={data.success ? "success" : "error"}
        text={data.success ? "success" : "failed"}
      />
    </svelte:fragment>
  </ListItem>

  <!-- TYPE -->
  <ListItem tooltipText="The ID of the transaction">
    <svelte:fragment slot="term">Type</svelte:fragment>
    <svelte:fragment slot="definition"
      ><Badge text={data.method} /></svelte:fragment
    >
  </ListItem>
</DetailList>
