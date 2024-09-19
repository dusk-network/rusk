<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiArrowRight } from "@mdi/js";
  import { AppAnchorButton, DataCard, ListItem } from "$lib/components";
  import {
    Card,
    ProgressBar,
    RelativeTime,
    Switch,
  } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    calculateAdaptiveCharCount,
    makeClassName,
    middleEllipsis,
  } from "$lib/dusk/string";
  import { onMount } from "svelte";
  import "./BlockDetails.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Block} */
  export let data;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {String | null} */
  export let payload;

  const formatter = createValueFormatter("en");

  /** @type {Number} */
  let screenWidth = window.innerWidth;

  /** @type {Boolean} */
  let isPayloadToggled = false;

  $: classes = makeClassName(["block-details", className]);

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
  title="Block Details"
>
  <dl class="block-details__list">
    <!-- HEIGHT -->
    <ListItem
      tooltipText="The height of the block indicates the length of the block chain and is increased with each additional block"
    >
      <svelte:fragment slot="term">height</svelte:fragment>
      <svelte:fragment slot="definition">
        <AppAnchorButton
          className="block-details__list-anchor"
          href="/blocks/block?id={data.header.prevblockhash}"
          icon={{ path: mdiArrowLeft }}
          disabled={!data.header.prevblockhash || data.header.height === 0}
        />
        {formatter(data.header.height)}
        <AppAnchorButton
          className="block-details__list-anchor"
          href="/blocks/block?id={data.header.nextblockhash}"
          icon={{ path: mdiArrowRight }}
          disabled={!data.header.nextblockhash}
        />
      </svelte:fragment>
    </ListItem>

    <!-- BLOCK HASH -->
    <ListItem tooltipText="The hash for the header of the block">
      <svelte:fragment slot="term">hash</svelte:fragment>
      <svelte:fragment slot="definition"
        >{middleEllipsis(
          data.header.hash,
          calculateAdaptiveCharCount(screenWidth, 320, 1920, 14, 66)
        )}</svelte:fragment
      >
    </ListItem>

    <!-- TIMESTAMP -->
    <ListItem tooltipText="The date and time the block was created">
      <svelte:fragment slot="term">timestamp</svelte:fragment>
      <RelativeTime
        autoRefresh={true}
        className="block-details__list-timestamp"
        date={data.header.date}
        slot="definition"
        ><svelte:fragment let:relativeTime
          >{`${data.header.date.toUTCString()} (${relativeTime})`}</svelte:fragment
        ></RelativeTime
      >
    </ListItem>

    <!-- TRANSACTIONS -->
    <ListItem tooltipText="The number of transactions included in the block">
      <svelte:fragment slot="term">transactions</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(data.transactions.data.length)}</svelte:fragment
      >
    </ListItem>

    <!-- BLOCK FEES PAID -->
    <ListItem
      tooltipText="The total fees paid for the transactions in the block"
    >
      <svelte:fragment slot="term">block fees paid</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.header.feespaid))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- BLOCK REWARD -->
    <ListItem tooltipText="The reward allocated to the block generator">
      <svelte:fragment slot="term">block reward</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.header.reward))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- BLOCK GAS LIMIT -->
    <ListItem tooltipText="The block gas limit">
      <svelte:fragment slot="term">block gas limit</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(data.transactions.stats.gasLimit)}</svelte:fragment
      >
    </ListItem>

    <!-- GAS USED -->
    <ListItem tooltipText="The amount of gas used generating the block">
      <svelte:fragment slot="term">gas used</svelte:fragment>
      <svelte:fragment slot="definition">
        {formatter(data.transactions.stats.gasUsed)}

        <ProgressBar
          currentPercentage={(data.transactions.stats.gasUsed /
            data.transactions.stats.gasLimit) *
            100}
          className="block-details__gas-used"
          ariaLabel="Gas Used"
        />
      </svelte:fragment>
    </ListItem>

    <!-- AVERAGE GAS PRICE -->
    <ListItem
      tooltipText="The average gas price for the transactions within the block"
    >
      <svelte:fragment slot="term">average gas price</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(data.transactions.stats.averageGasPrice)}</svelte:fragment
      >
    </ListItem>

    <!-- STATE ROOT HASH -->
    <ListItem tooltipText="The state root hash">
      <svelte:fragment slot="term">state root hash</svelte:fragment>
      <span class="block-details__state-hash" slot="definition"
        >{middleEllipsis(
          data.header.statehash,
          calculateAdaptiveCharCount(screenWidth, 320, 1920, 14, 66)
        )}</span
      >
    </ListItem>

    <!-- HEADER -->
    <ListItem tooltipText="The block header information">
      <svelte:fragment slot="term">
        header

        <Switch
          className="block-details__payload-switch"
          onSurface={true}
          bind:value={isPayloadToggled}
        />
      </svelte:fragment>

      <svelte:fragment slot="definition">
        {#if isPayloadToggled}
          <Card onSurface={true} className="block-details__payload">
            <pre>{payload
                ? JSON.stringify(JSON.parse(payload), null, 2)
                : "---"}</pre>
          </Card>
        {/if}
      </svelte:fragment>
    </ListItem>
  </dl>
</DataCard>
