<script>
  import { mdiShieldLock, mdiShieldOutline } from "@mdi/js";
  import { Badge, Icon } from "$lib/dusk/components";
  import "./TransactionType.css";

  export let data;

  /** @type {boolean} */
  export let displayTooltips = false;
</script>

<div class="transaction-type">
  <Icon
    className="transaction-type__icon"
    data-tooltip-disabled={!displayTooltips}
    data-tooltip-id="main-tooltip"
    data-tooltip-text={data.txtype}
    data-tooltip-place="top"
    data-tooltip-type="info"
    path={data.txtype.toLowerCase() === "moonlight"
      ? mdiShieldLock
      : mdiShieldOutline}
    size="large"
  />
  <Badge text={data.method} />
</div>
