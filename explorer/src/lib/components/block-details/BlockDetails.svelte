<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiArrowRight } from "@mdi/js";
  import { AppAnchor, DataCard, ListItem } from "$lib/components";
  import { Icon, ProgressBar } from "$lib/dusk/components";
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

  /** @type {string}*/
  let blockHeight;

  $: classes = makeClassName(["block-details", className]);

  $: {
    if (data) {
      blockHeight = formatter(data.header.height);
    }
  }

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
  title="Block Details - #{blockHeight}"
  button={{ action: () => history.back(), label: "Back" }}
>
  <dl class="block-details__list">
    <!-- BLOCK HASH -->
    <ListItem tooltipText="The hash for the header of the block">
      <svelte:fragment slot="term">block hash</svelte:fragment>
      <svelte:fragment slot="definition"
        >{middleEllipsis(
          data.header.hash,
          calculateAdaptiveCharCount(screenWidth, 320, 1920, 14, 66)
        )}</svelte:fragment
      >
    </ListItem>

    <!-- HEIGHT -->
    <ListItem
      tooltipText="The height of the block indicates the length of the block chain and is increased with each additional block"
    >
      <svelte:fragment slot="term">height</svelte:fragment>
      <svelte:fragment slot="definition">
        <AppAnchor
          className="block-details__list-anchor"
          href="/blocks/block?id={data.header.prevblockhash}"
        >
          <Icon path={mdiArrowLeft} />
        </AppAnchor>
        {formatter(data.header.height)}
        <AppAnchor
          className="block-details__list-anchor"
          href="/blocks/block?id={data.header.nextblockhash}"
        >
          <Icon path={mdiArrowRight} />
        </AppAnchor>
      </svelte:fragment>
    </ListItem>

    <!-- TIMESTAMP -->
    <ListItem tooltipText="The date and time the block was created">
      <svelte:fragment slot="term">timestamp</svelte:fragment>
      <time
        datetime={data.header.date.toISOString()}
        class="block-details__list-timestamp"
        slot="definition"
      >
        {getRelativeTimeString(data.header.date, "long")}
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
        >{formatter(luxToDusk(data.transactions.stats.gasLimit))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- GAS USED -->
    <ListItem tooltipText="The amount of gas used generating the block">
      <svelte:fragment slot="term">gas used</svelte:fragment>
      <svelte:fragment slot="definition">
        {formatter(luxToDusk(data.transactions.stats.gasUsed))} DUSK

        <ProgressBar
          currentPercentage={(data.transactions.stats.gasUsed /
            data.transactions.stats.gasLimit) *
            100}
          className="block-details__gas-used"
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
