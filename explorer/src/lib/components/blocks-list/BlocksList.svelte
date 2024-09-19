<svelte:options immutable={true} />

<script>
  import { AppAnchor, DetailList, ListItem } from "$lib/components";
  import { ProgressBar, RelativeTime } from "$lib/dusk/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import { luxToDusk } from "$lib/dusk/currency";
  import "./BlocksList.css";

  /** @type {boolean} */
  export let autoRefreshTime = false;

  /** @type {Block} */
  export let data;

  /** @type {Boolean} */
  export let displayTooltips = false;

  const formatter = createValueFormatter("en");
</script>

<DetailList>
  <!-- HEIGHT -->
  <ListItem
    tooltipText={displayTooltips
      ? "The height of the block indicates the length of the block chain and is increased with each additional block"
      : ""}
  >
    <svelte:fragment slot="term">block</svelte:fragment>
    <svelte:fragment slot="definition"
      ><AppAnchor
        className="block-details__list-link"
        href={`/blocks/block?id=${data.header.hash}`}
        >{formatter(data.header.height)}</AppAnchor
      ></svelte:fragment
    >
  </ListItem>

  <!-- TIMESTAMP -->
  <ListItem
    tooltipText={displayTooltips
      ? "Time elapsed since the block was created"
      : ""}
  >
    <svelte:fragment slot="term">relative time</svelte:fragment>
    <RelativeTime
      autoRefresh={autoRefreshTime}
      className="block-details__list-timestamp"
      date={data.header.date}
      slot="definition"
    />
  </ListItem>

  <!-- AVERAGE GAS PRICE -->
  <ListItem
    tooltipText={displayTooltips
      ? "The average gas price for the transactions within the block"
      : ""}
  >
    <svelte:fragment slot="term">average gas price</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(data.transactions.stats.averageGasPrice)}
    </svelte:fragment>
  </ListItem>

  <!-- GAS USED -->
  <ListItem
    tooltipText={displayTooltips
      ? "The amount of gas used generating the block"
      : ""}
  >
    <svelte:fragment slot="term">gas used</svelte:fragment>
    <svelte:fragment slot="definition">
      <ProgressBar
        currentPercentage={(data.transactions.stats.gasUsed /
          data.transactions.stats.gasLimit) *
          100}
        className="blocks-list__gas-used"
        ariaLabel="Gas Used"
      />
    </svelte:fragment>
  </ListItem>

  <!-- TRANSACTIONS -->
  <ListItem
    tooltipText={displayTooltips
      ? "The number of transactions included in the block"
      : ""}
  >
    <svelte:fragment slot="term">txn(s)</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(data.transactions.data.length)}
    </svelte:fragment>
  </ListItem>

  <!-- BLOCK REWARD -->
  <ListItem
    tooltipText={displayTooltips
      ? "The reward allocated to the block generator"
      : ""}
  >
    <svelte:fragment slot="term">rewards</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(luxToDusk(data.header.reward))} DUSK
    </svelte:fragment>
  </ListItem>
</DetailList>
