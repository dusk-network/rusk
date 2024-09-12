<script>
  import { mdiShieldLock, mdiShieldOutline } from "@mdi/js";
  import { Badge, Icon } from "$lib/dusk/components";
  import "./TransactionType.css";

  export let data;
</script>

<div class="transaction-type">
  <Icon
    path={data.txtype.toLowerCase() === "moonlight"
      ? mdiShieldLock
      : mdiShieldOutline}
    size="large"
    data-tooltip-id="main-tooltip"
    data-tooltip-text={data.txtype}
    data-tooltip-place="top"
    data-tooltip-type="info"
  />
  <Badge text={data.method} />
</div>
