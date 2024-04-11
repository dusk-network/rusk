<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiArrowRight } from "@mdi/js";
  import { AppAnchor, ListItem } from "$lib/components";
  import { Card, Icon, ProgressBar } from "$lib/dusk/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import { luxToDusk } from "$lib/dusk/currency";
  import {
    calculateAdaptiveCharCount,
    getRelativeTimeString,
    middleEllipsis,
  } from "$lib/dusk/string";
  import { onMount } from "svelte";
  import "./BlockDetails.css";

  /** @type {*} */
  export let data;

  const formatter = createValueFormatter("en");

  /** @type {number} */
  let screenWidth = window.innerWidth;

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(
      document.body.getElementsByClassName("details-list__definition")[0]
    );

    return () => resizeObserver.disconnect();
  });
</script>

<Card className="block-details">
  <header slot="header" class="block-details__header">
    <h3 class="block-details__header-heading">
      Block Details <span class="block-details__block-height">
        - #{formatter(34526)}</span
      >
    </h3>
    <button type="button" on:click={() => history.back()}>Back</button>
  </header>
  <dl class="block-details__list">
    <!-- BLOCK HASH -->
    <ListItem
      tooltipId="blockHash"
      tooltipText="The hash for the header of the block"
    >
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
      tooltipId="height"
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
    <ListItem
      tooltipId="timestamp"
      tooltipText="The date and time the block was created"
    >
      <svelte:fragment slot="term">timestamp</svelte:fragment>
      <time
        datetime={new Date(data.header.ts * 1000).toISOString()}
        class="block-details__list-timestamp"
        slot="definition"
      >
        {getRelativeTimeString(new Date(data.header.ts * 1000), "long")}
      </time>
    </ListItem>

    <!-- TRANSACTIONS -->
    <ListItem
      tooltipId="transactions"
      tooltipText="The number of transactions included in the block"
    >
      <svelte:fragment slot="term">transactions</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(data.transactions.data.length)}</svelte:fragment
      >
    </ListItem>

    <!-- BLOCK FEES PAID -->
    <ListItem
      tooltipId="blockFees"
      tooltipText="The total fees paid for the transactions in the block"
    >
      <svelte:fragment slot="term">block fees paid</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.header.feespaid))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- BLOCK REWARD -->
    <ListItem
      tooltipId="blockReward"
      tooltipText="The reward allocated to the block generator"
    >
      <svelte:fragment slot="term">block reward</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.header.reward))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- BLOCK GAS LIMIT -->
    <ListItem tooltipId="blockGasLimit" tooltipText="The block gas limit">
      <svelte:fragment slot="term">block gas limit</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.transactions.stats.gasLimit))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- GAS USED -->
    <ListItem
      tooltipId="gasUsed"
      tooltipText="The amount of gas used generating the block"
    >
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
      tooltipId="averageFeePaid"
      tooltipText="The average fee paid for the transactions within the block"
    >
      <svelte:fragment slot="term">average fee paid</svelte:fragment>
      <svelte:fragment slot="definition"
        >{formatter(luxToDusk(data.transactions.stats.averageGasPrice))} DUSK</svelte:fragment
      >
    </ListItem>

    <!-- STATE ROOT HASH -->
    <ListItem tooltipId="stateRootHash" tooltipText="The state root hash">
      <svelte:fragment slot="term">state root hash</svelte:fragment>
      <span class="block-details__state-hash" slot="definition"
        >{middleEllipsis(
          data.header.statehash,
          calculateAdaptiveCharCount(screenWidth, 320, 1920, 14, 66)
        )}</span
      >
    </ListItem>
  </dl>
</Card>
