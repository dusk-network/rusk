<svelte:options immutable={true} />

<script>
  import {
    AppAnchor,
    DataGuard,
    DetailList,
    ListItem,
    TransactionStatus,
    TransactionType,
  } from "$lib/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { RelativeTime } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { onMount } from "svelte";

  /** @type {boolean} */
  export let autoRefreshTime = false;

  /** @type {Transaction}*/
  export let data;

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
  {#if data.from}
    <ListItem
      tooltipText={displayTooltips ? "The sender of the transactions" : ""}
    >
      <svelte:fragment slot="term">From</svelte:fragment>
      <svelte:fragment slot="definition">
        {middleEllipsis(
          data.from,
          calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 25)
        )}
      </svelte:fragment>
    </ListItem>
  {/if}
  {#if data.to}
    <ListItem
      tooltipText={displayTooltips ? "The recipient of the transaction" : ""}
    >
      <svelte:fragment slot="term">To</svelte:fragment>
      <svelte:fragment slot="definition">
        {middleEllipsis(
          data.to,
          calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 25)
        )}
      </svelte:fragment>
    </ListItem>
  {/if}

  <!-- TRANSACTION ID -->
  <ListItem tooltipText={displayTooltips ? "The ID of the transaction" : ""}>
    <svelte:fragment slot="term">ID</svelte:fragment>
    <svelte:fragment slot="definition">
      <AppAnchor
        className="transaction-details__list-link"
        href={`/transactions/transaction?id=${data.txid}`}
        >{middleEllipsis(
          data.txid,
          calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 25)
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
    <RelativeTime
      autoRefresh={autoRefreshTime}
      date={data.date}
      className="transaction-details__list-timestamp"
      slot="definition"
    />
  </ListItem>

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
      <TransactionStatus
        className="explorer-badge"
        errorMessage={data.txerror}
        showErrorTooltip={autoRefreshTime}
      />
    </svelte:fragment>
  </ListItem>

  <!-- TYPE -->
  <ListItem tooltipText={displayTooltips ? "The transaction type" : ""}>
    <svelte:fragment slot="term">Type</svelte:fragment>
    <svelte:fragment slot="definition"
      ><DataGuard data={data.method && data.txtype}>
        <TransactionType {data} {displayTooltips} />
      </DataGuard></svelte:fragment
    >
  </ListItem>
</DetailList>
