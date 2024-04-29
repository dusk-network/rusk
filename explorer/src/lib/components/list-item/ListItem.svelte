<svelte:options immutable={true} />

<script>
  import { Icon } from "$lib/dusk/components";
  import { mdiInformationOutline } from "@mdi/js";
  import { getContext } from "svelte";
  import contexts from "$lib/contexts";

  import "./ListItem.css";

  /** @type {string} */
  export let tooltipText = "";

  const context = getContext("DUSK:list-item:context") || "";
</script>

<dt
  class="details-list__term"
  class:details-list__term--table={context === contexts.LIST_ITEM.DETAIL_LIST}
>
  {#if tooltipText}
    <Icon
      className="detail-list__help"
      path={mdiInformationOutline}
      size="normal"
      data-tooltip-id="main-tooltip"
      data-tooltip-text={tooltipText}
      data-tooltip-place="top"
      data-tooltip-type="info"
    />
  {/if}
  <slot name="term" />
</dt>
<dd
  class="details-list__definition"
  class:details-list__definition--table={context ===
    contexts.LIST_ITEM.DETAIL_LIST}
>
  <slot name="definition" />
</dd>
