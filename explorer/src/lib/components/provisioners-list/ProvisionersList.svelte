<svelte:options immutable={true} />

<script>
  import { DetailList, ListItem } from "$lib/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import { luxToDusk } from "$lib/dusk/currency";
  import { middleEllipsis } from "$lib/dusk/string";

  /** @type {HostProvisioner} */
  export let data;

  /** @type {Boolean} */
  export let displayTooltips = false;

  const HASH_CHARS_LENGTH = 10;

  const formatter = createValueFormatter("en");
</script>

<DetailList>
  <!-- STAKING ADDRESS -->
  <ListItem tooltipText={displayTooltips ? "The staking address used" : ""}>
    <svelte:fragment slot="term">staking address</svelte:fragment>
    <svelte:fragment slot="definition"
      >{middleEllipsis(data.key, HASH_CHARS_LENGTH)}</svelte:fragment
    >
  </ListItem>

  <!-- STAKED AMOUNT -->
  <ListItem tooltipText={displayTooltips ? "The staked amount" : ""}>
    <svelte:fragment slot="term">staked amount</svelte:fragment>
    <svelte:fragment slot="definition"
      >{formatter(luxToDusk(data.amount))} DUSK</svelte:fragment
    >
  </ListItem>

  <!-- RECLAIMABLE STAKED AMOUNT -->
  <ListItem
    tooltipText={displayTooltips ? "The reclaimable staked amount" : ""}
  >
    <svelte:fragment slot="term">Reclaimable Staked Amount</svelte:fragment>
    <svelte:fragment slot="definition"
      >{formatter(data.locked_amt)} DUSK</svelte:fragment
    >
  </ListItem>

  <!-- SLASHES -->
  <ListItem tooltipText={displayTooltips ? "Slashes" : ""}>
    <svelte:fragment slot="term">slashes</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(data.faults)}
    </svelte:fragment>
  </ListItem>

  <!-- HARD SLASHES -->
  <ListItem tooltipText={displayTooltips ? "Hard slashes" : ""}>
    <svelte:fragment slot="term">hard slashes</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(data.hard_faults)}
    </svelte:fragment>
  </ListItem>

  <!-- REWARD -->
  <ListItem tooltipText={displayTooltips ? "The accumulated reward" : ""}>
    <svelte:fragment slot="term">accumulated reward</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(luxToDusk(data.reward))} DUSK
    </svelte:fragment>
  </ListItem>
</DetailList>
