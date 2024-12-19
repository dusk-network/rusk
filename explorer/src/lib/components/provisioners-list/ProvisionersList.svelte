<svelte:options immutable={true} />

<script>
  import { onMount } from "svelte";
  import { ownPairs } from "lamb";

  import { Badge } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { createValueFormatter } from "$lib/dusk/value";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { DetailList, ListItem } from "$lib/components";

  import "./ProvisionersList.css";

  /** @type {HostProvisioner} */
  export let data;

  /** @type {Boolean} */
  export let displayTooltips = false;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const formatter = createValueFormatter("en");
  const [ownerType, ownerValue] = ownPairs(data.owner)[0];

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  $: provisionerKey = middleEllipsis(
    data.key,
    calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 25)
  );
</script>

<DetailList className="provisioners-list">
  <!-- STAKING ADDRESS -->
  <ListItem tooltipText={displayTooltips ? "The staking address used" : ""}>
    <svelte:fragment slot="term">Staking address</svelte:fragment>
    <svelte:fragment slot="definition"
      ><span class="provisioners-list__staking-address">{provisionerKey}</span
      ></svelte:fragment
    >
  </ListItem>

  <!-- OWNER -->
  <ListItem tooltipText={displayTooltips ? "The provisioner's owner" : ""}>
    <svelte:fragment slot="term">Owner</svelte:fragment>
    <svelte:fragment slot="definition">
      <Badge
        data-tooltip-id="provisioners-tooltip"
        data-tooltip-text={ownerType === "Account"
          ? provisionerKey
          : ownerValue}
        text={ownerType}
      />
    </svelte:fragment>
  </ListItem>

  <!-- LOCKED STAKED AMOUNT -->
  <ListItem tooltipText={displayTooltips ? "The locked stake amount" : ""}>
    <svelte:fragment slot="term">Locked Stake Amount</svelte:fragment>
    <svelte:fragment slot="definition"
      >{formatter(luxToDusk(data.locked_amt))} DUSK</svelte:fragment
    >
  </ListItem>

  <!-- RECLAIMABLE STAKED AMOUNT -->
  <ListItem tooltipText={displayTooltips ? "The reclaimable stake amount" : ""}>
    <svelte:fragment slot="term">Reclaimable Stake Amount</svelte:fragment>
    <svelte:fragment slot="definition"
      >{formatter(luxToDusk(data.amount))} DUSK</svelte:fragment
    >
  </ListItem>

  <!-- SLASHES -->
  <ListItem tooltipText={displayTooltips ? "Soft slashes" : ""}>
    <svelte:fragment slot="term">Soft Slashes</svelte:fragment>
    <svelte:fragment slot="definition">
      {formatter(data.faults)}
    </svelte:fragment>
  </ListItem>

  <!-- HARD SLASHES -->
  <ListItem tooltipText={displayTooltips ? "Hard slashes" : ""}>
    <svelte:fragment slot="term">Hard Slashes</svelte:fragment>
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
