<svelte:options immutable={true} />

<script>
  import { AppAnchor, DetailList, ListItem } from "$lib/components";
  import { Badge, ProgressBar } from "$lib/dusk/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import { getRelativeTimeString } from "$lib/dusk/string";
  import { luxToDusk } from "$lib/dusk/currency";
  import "./BlocksList.css";

  /** @type {Block} */
  export let data;

  const formatter = createValueFormatter("en");
</script>

<DetailList>
  <!-- HEIGHT -->
  <ListItem
    tooltipText="The height of the block indicates the length of the block chain and is increased with each additional block"
  >
    <svelte:fragment slot="term"># block</svelte:fragment>
    <svelte:fragment slot="definition"
      ><AppAnchor
        className="block-details__list-link"
        href={`/blocks/block?id=${data.header.hash}`}
        >{formatter(data.header.height)}</AppAnchor
      ></svelte:fragment
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
      {getRelativeTimeString(data.header.date, "long")}
    </time>
  </ListItem>

  <!-- AVERAGE FEE PAID -->
  <ListItem
    tooltipText="The average fee paid for the transactions within the block"
  >
    <svelte:fragment slot="term">average fee paid</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(luxToDusk(data.transactions.stats.averageGasPrice))} DUSK
    </svelte:fragment>
  </ListItem>

  <!-- GAS USED -->
  <ListItem tooltipText="The amount of gas used generating the block">
    <svelte:fragment slot="term">gas used</svelte:fragment>
    <svelte:fragment slot="definition">
      <ProgressBar
        currentPercentage={(data.transactions.stats.gasUsed /
          data.transactions.stats.gasLimit) *
          100}
        className="blocks-list__gas-used"
      />
    </svelte:fragment>
  </ListItem>

  <!-- TRANSACTIONS -->
  <ListItem tooltipText="The number of transactions included in the block">
    <svelte:fragment slot="term">txn(s)</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(data.transactions.data.length)}
    </svelte:fragment>
  </ListItem>

  <!-- BLOCK REWARD -->
  <ListItem tooltipText="The reward allocated to the block generator">
    <svelte:fragment slot="term">rewards</svelte:fragment>
    <svelte:fragment slot="definition">
      <Badge
        variant="alt"
        text={`${formatter(luxToDusk(data.header.reward))} Dusk`}
      />
    </svelte:fragment>
  </ListItem>
</DetailList>
