<script>
  import { logo } from "$lib/dusk/icons";
  import { Icon } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";

  /** @type {bigint} */
  export let fee;

  /** @type {(amount: number) => string} */
  export let formatter;
</script>

<div class="gas-fee">
  <span>Fee:</span>
  <div class="gas-fee__amount">
    <div class="gas-fee__amount-value">
      <span>max</span> <span>{formatter(luxToDusk(fee))}</span>
    </div>
    <Icon
      className="dusk-amount__icon"
      path={logo}
      data-tooltip-id="main-tooltip"
      data-tooltip-text="DUSK"
    />
  </div>
</div>

<style lang="postcss">
  .gas-fee {
    display: inline-flex;
    justify-content: flex-start;
    align-items: center;
    gap: var(--small-gap);
    flex-wrap: wrap;

    &__amount {
      display: inline-flex;
      gap: var(--small-gap);
      align-items: center;
    }

    &__amount-value {
      font-weight: 600;
      display: flex;
      gap: var(--small-gap);

      & > span:first-of-type {
        font-weight: initial;
      }
    }
  }

  :global(.dusk-amount__icon) {
    width: 1em;
    height: 1em;
    flex-shrink: 0;
  }
</style>
