<script>
  import { mdiShieldLock, mdiShieldLockOpenOutline } from "@mdi/js";
  import { Badge, Icon } from "$lib/dusk/components";
  import "./TransactionType.css";
  import { page } from "$app/stores";

  export let data;

  /** @type {boolean} */
  export let displayTooltips = false;

  const BADGE_TEXT_MAX_LENGTH = 20;
  const isAccountsPage = $page.url.pathname.includes("accounts");

  /**
   * @param {Transaction} transaction
   * @returns {"in" | "out" | "self" | undefined}
   */
  const getTransactionDirection = ({ from, to }) => {
    const key = $page.url.searchParams.get("key");
    if (key === from && key === to) {
      return "self";
    }
    if (key === from) {
      return "out";
    }
    if (key === to) {
      return "in";
    }
    return undefined;
  };
</script>

<div class="transaction-type">
  {#if !isAccountsPage}
    <Icon
      className="transaction-type__icon"
      data-tooltip-disabled={!displayTooltips}
      data-tooltip-id="main-tooltip"
      data-tooltip-text={data.txtype.toLowerCase() === "moonlight"
        ? "Public"
        : "Shielded"}
      data-tooltip-place="top"
      data-tooltip-type="info"
      path={data.txtype.toLowerCase() === "moonlight"
        ? mdiShieldLockOpenOutline
        : mdiShieldLock}
      size="large"
    />
  {/if}
  <Badge
    className="transaction-type__method-badge"
    text={data.method}
    maxlength={BADGE_TEXT_MAX_LENGTH}
  />
  {#if isAccountsPage}
    <Badge
      className="transaction-type__direction-badge"
      text={getTransactionDirection(data) ?? "N/A"}
    />
  {/if}
</div>

<style lang="postcss">
  :global(.transaction-type__method-badge) {
    width: 8.125rem;
  }

  :global(.transaction-type__direction-badge) {
    width: 5.625em;
  }
</style>
