<svelte:options immutable={true} />

<script>
  import { ListItem } from "$lib/components";
  import { Badge, Button, Card, Switch } from "$lib/dusk/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    createCurrencyFormatter,
    createFeeFormatter,
    luxToDusk,
  } from "$lib/dusk/currency";
  import {
    calculateAdaptiveCharCount,
    getRelativeTimeString,
    middleEllipsis,
  } from "$lib/dusk/string";
  import { onMount } from "svelte";
  import "./TransactionDetails.css";

  const formatter = createValueFormatter("en");
  const currencyFormatter = createCurrencyFormatter("en", "usd", 9);
  const feeFormatter = createFeeFormatter("en");

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {boolean} */
  let isPayloadToggled = false;

  /** @type {HTMLElement}*/
  let transactionList;

  /** @type {*} */
  export let data;

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(transactionList.children[1]);

    return () => resizeObserver.disconnect();
  });
</script>

<Card className="transaction-details">
  <header slot="header" class="transaction-details__header">
    <h3 class="transaction-details__header-heading">Transaction Details</h3>
    <Button on:click={() => history.back()} text="Back" variant="secondary" />
  </header>
  <dl class="transaction-details__list" bind:this={transactionList}>
    <!-- TRANSACTION ID -->
    <ListItem tooltipText="The ID of the transaction">
      <svelte:fragment slot="term">ID</svelte:fragment>
      <svelte:fragment slot="definition"
        >{middleEllipsis(
          data.txid,
          calculateAdaptiveCharCount(screenWidth, 320, 1920, 14, 66)
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
        datetime={new Date(data.blockts * 1000).toISOString()}
        class="transaction-details__list-timestamp"
        slot="definition"
      >
        {getRelativeTimeString(new Date(data.blockts * 1000), "long")}
        {new Date(data.blockts * 1000).toUTCString()}
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
            <svelte:fragment>Some text</svelte:fragment>
          </Card>
        {/if}
      </svelte:fragment>
    </ListItem>
  </dl>
</Card>
