<script>
  import { mdiShieldLock, mdiShieldLockOpenOutline } from "@mdi/js";
  import { Badge, Icon } from "$lib/dusk/components";
  import "./TransactionType.css";

  export let data;

  /** @type {boolean} */
  export let displayTooltips = false;

  const BADGE_TEXT_MAX_LENGTH = 20;
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
      ? mdiShieldLockOpenOutline
      : mdiShieldLock}
    size="large"
  />
  <Badge text={data.method} maxlength={BADGE_TEXT_MAX_LENGTH} />
</div>
