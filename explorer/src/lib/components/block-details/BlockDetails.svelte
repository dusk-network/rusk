<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiArrowRight } from "@mdi/js";
  import {
    AppAnchorButton,
    DataCard,
    ListItem,
    Rerender,
  } from "$lib/components";
  import { ProgressBar } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    calculateAdaptiveCharCount,
    getRelativeTimeString,
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

  const formatter = createValueFormatter("en");

  /** @type {Number} */
  let screenWidth = window.innerWidth;

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
      <time
        datetime={data.header.date.toISOString()}
        class="block-details__list-timestamp"
        slot="definition"
      >
        <Rerender>
          {`${data.header.date.toUTCString()} (${getRelativeTimeString(data.header.date, "long")})`}
        </Rerender>
      </time>
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

    <!-- AVERAGE FEE PAID -->
    <ListItem
      tooltipText="The average fee paid for the transactions within the block"
    >
      <svelte:fragment slot="term">average fee paid</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.transactions.stats.averageGasPrice))} DUSK</svelte:fragment
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
  </dl>
</DataCard>
