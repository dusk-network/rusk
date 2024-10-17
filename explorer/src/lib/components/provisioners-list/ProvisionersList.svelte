<svelte:options immutable={true} />

<script>
  import { DetailList, ListItem } from "$lib/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import { luxToDusk } from "$lib/dusk/currency";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { onMount } from "svelte";

  /** @type {HostProvisioner} */
  export let data;

  /** @type {Boolean} */
  export let displayTooltips = false;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const formatter = createValueFormatter("en");

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
  <!-- STAKING ADDRESS -->
  <ListItem tooltipText={displayTooltips ? "The staking address used" : ""}>
    <svelte:fragment slot="term">staking address</svelte:fragment>
    <svelte:fragment slot="definition"
      >{middleEllipsis(
        data.key,
        calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 30)
      )}</svelte:fragment
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
