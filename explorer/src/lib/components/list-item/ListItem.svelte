<svelte:options immutable={true} />

<script>
  import { Icon, Tooltip } from "$lib/dusk/components";
  import { mdiInformationOutline } from "@mdi/js";

  import "./ListItem.css";

  /** @type {string} */
  export let tooltipText = "";

  /**@type {string}*/
  export let tooltipId;
</script>

<dt class="details-list__term">
  {#if tooltipText}
    <Icon
      className="detail-list__help"
      path={mdiInformationOutline}
      size="normal"
      data-tooltip-id={tooltipId}
      data-tooltip-text={tooltipText}
      data-tooltip-place="top"
      data-tooltip-type="info"
    />
  {/if}
  <slot name="term" />
</dt>
<dd class="details-list__definition">
  <slot name="definition" />
</dd>
<Tooltip id={tooltipId} />
