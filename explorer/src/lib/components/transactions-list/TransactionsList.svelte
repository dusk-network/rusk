<svelte:options immutable={true} />

<script>
  import { onMount } from "svelte";
  import { CopyButton, RelativeTime } from "$lib/dusk/components";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { createValueFormatter } from "$lib/dusk/value";
  import { luxToDusk } from "$lib/dusk/currency";
  import {
    AppAnchor,
    DataGuard,
    DetailList,
    ListItem,
    TransactionStatus,
    TransactionType,
  } from "$lib/components";
  import { addressCharPropertiesDefaults } from "$lib/constants";

  import "./TransactionsList.css";

  /** @type {boolean} */
  export let autoRefreshTime = false;

  /** @type {Transaction}*/
  export let data;

  /** @type {Boolean} */
  export let displayTooltips = false;

  /** @type {"compact" | "full"} */
  export let mode;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const formatter = createValueFormatter("en");

  const { minScreenWidth, maxScreenWidth, minCharCount, maxCharCount } =
    addressCharPropertiesDefaults;

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
          calculateAdaptiveCharCount(
            screenWidth,
            minScreenWidth,
            maxScreenWidth,
            minCharCount,
            maxCharCount
          )
        )}</AppAnchor
      >
      <CopyButton
        name="Transaction's ID"
        rawValue={data.txid}
        variant="secondary"
      />
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
