<svelte:options immutable={true} />

<script>
  import { DataCard, DataGuard, ListItem } from "$lib/components";
  import { Badge, Card, Switch } from "$lib/dusk/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    createCurrencyFormatter,
    createFeeFormatter,
    luxToDusk,
  } from "$lib/dusk/currency";
  import {
    calculateAdaptiveCharCount,
    getRelativeTimeString,
    makeClassName,
    middleEllipsis,
  } from "$lib/dusk/string";
  import { onMount } from "svelte";
  import "./TransactionDetails.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Transaction} */
  export let data;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {String | null} */
  export let payload;

  const formatter = createValueFormatter("en");
  const currencyFormatter = createCurrencyFormatter("en", "usd", 9);
  const feeFormatter = createFeeFormatter("en");

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {boolean} */
  let isPayloadToggled = false;

  $: classes = makeClassName(["transaction-details", className]);

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });
</script>

<DataCard
  on:retry
  {data}
  {error}
  {loading}
  className={classes}
  title="Transaction Details"
  headerButtonDetails={{
    action: () => history.back(),
    disabled: false,
    label: "Back",
  }}
>
  <dl class="transaction-details__list">
    <!-- TRANSACTION ID -->
    <ListItem tooltipText="The ID of the transaction">
      <svelte:fragment slot="term">ID</svelte:fragment>
      <svelte:fragment slot="definition"
        >{middleEllipsis(
          data.txid,
          calculateAdaptiveCharCount(screenWidth, 320, 1400, 14, 40)
        )}</svelte:fragment
      >
    </ListItem>

    <!-- BLOCK HEIGHT -->
    <ListItem tooltipText="The block height this transaction belongs to">
      <svelte:fragment slot="term">block height</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(data.blockheight)}</svelte:fragment
      >
    </ListItem>

    <!-- STATUS -->
    <ListItem tooltipText="The transaction status">
      <svelte:fragment slot="term">Status</svelte:fragment>
      <svelte:fragment slot="definition"
        ><Badge
          className="transaction-details__status"
          variant={data.success ? "success" : "error"}
          text={data.success ? "success" : "failed"}
        /></svelte:fragment
      >
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
        {data.date.toUTCString()}
      </time>
    </ListItem>

    <!-- TYPE -->
    <ListItem tooltipText="The transaction type">
      <svelte:fragment slot="term">type</svelte:fragment>
      <svelte:fragment slot="definition"
        ><Badge
          className="transaction-details__type"
          text={data.method}
        /></svelte:fragment
      >
    </ListItem>

    <!-- TRANSACTION FEE -->
    <ListItem tooltipText="The fee paid for the transaction">
      <svelte:fragment slot="term">transaction fee</svelte:fragment>
      <svelte:fragment slot="definition"
        >{`${feeFormatter(luxToDusk(data.feepaid))} DUSK (${currencyFormatter(luxToDusk(data.feepaid) * 0.5)})`}</svelte:fragment
      >
    </ListItem>

    <!-- GAS PRICE -->
    <ListItem tooltipText="The transaction gas price">
      <svelte:fragment slot="term">gas price</svelte:fragment>
      <svelte:fragment slot="definition"
        >{`${feeFormatter(luxToDusk(data.gasprice))} DUSK (${currencyFormatter(luxToDusk(data.gasprice) * 0.5)})`}</svelte:fragment
      >
    </ListItem>

    <!-- GAS LIMIT -->
    <ListItem tooltipText="The transaction gas limit">
      <svelte:fragment slot="term">transaction gas limit</svelte:fragment>
      <svelte:fragment slot="definition"
        >{feeFormatter(luxToDusk(data.gaslimit))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- GAS SPENT -->
    <ListItem tooltipText="The amount of gas spent generating the transaction">
      <svelte:fragment slot="term">gas spent</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.gasspent))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- PAYLOAD -->
    <ListItem tooltipText="The payload">
      <svelte:fragment slot="term">
        payload

        <Switch
          className="transaction-details__payload-switch"
          onSurface={true}
          bind:value={isPayloadToggled}
        />
      </svelte:fragment>

      <svelte:fragment slot="definition">
        {#if isPayloadToggled}
          <Card onSurface={true} className="transaction-details__payload">
            <svelte:fragment>
              <DataGuard data={payload}>
                {payload}
              </DataGuard>
            </svelte:fragment>
          </Card>
        {/if}
      </svelte:fragment>
    </ListItem>
  </dl>
</DataCard>
