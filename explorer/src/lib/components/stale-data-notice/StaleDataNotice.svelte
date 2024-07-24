<script>
  import { mdiAlertOutline } from "@mdi/js";
  import { Icon } from "$lib/dusk/components";
  import { marketDataStore } from "$lib/stores";

  $: tooltipText = `Stale Data \nLast Update: ${$marketDataStore.lastUpdate?.toUTCString()}`;
  $: isStaleData = $marketDataStore.lastUpdate && marketDataStore.isDataStale();
</script>

{#if isStaleData}
  <Icon
    path={mdiAlertOutline}
    size="normal"
    data-tooltip-id="main-tooltip"
    data-tooltip-text={tooltipText}
    data-tooltip-place="top"
    data-tooltip-type="warning"
  />
{/if}
