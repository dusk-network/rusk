<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";
  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { Card, Icon } from "$lib/dusk/components";
  import { mdiShieldLock, mdiShieldLockOpenOutline } from "@mdi/js";
  import { logo } from "$lib/dusk/icons";
  import { createNumberFormatter } from "$lib/dusk/number";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {string} */
  export let fiatCurrency;

  /** @type {number | undefined} */
  export let fiatPrice = undefined;

  /**
   * A BCP 47 language tag.
   * @type {string}
   */
  export let locale;

  /** @type {string} */
  export let tokenCurrency;

  /** @type {bigint} */
  export let shieldedAmount;

  /** @type {bigint} */
  export let unshieldedAmount;

  $: totalBalance = luxToDusk(shieldedAmount + unshieldedAmount);

  $: shieldedPercentage = (luxToDusk(shieldedAmount) / totalBalance) * 100;
  $: unshieldedPercentage = (luxToDusk(unshieldedAmount) / totalBalance) * 100;

  $: classes = makeClassName(["dusk-balance", className]);

  $: duskFormatter = createCurrencyFormatter(locale, tokenCurrency, 9);
  $: fiatFormatter = createCurrencyFormatter(locale, fiatCurrency, 2);
  $: numberFormatter = createNumberFormatter(locale, 2);
</script>

<article {...$$restProps} class={classes}>
  <header class="dusk-balance__header">
    <h2 class="sr-only">Your Balance:</h2>
  </header>
  <p class="dusk-balance__dusk">
    <strong>{duskFormatter(totalBalance)}</strong>
    <strong>{tokenCurrency}</strong>
  </p>
  <p
    class="dusk-balance__fiat"
    class:dusk-balance__fiat--visible={fiatPrice !== undefined}
  >
    <strong>
      {fiatFormatter(fiatPrice ? fiatPrice * totalBalance : 0)}
    </strong>
  </p>

  <Card className="dusk-balance__usage-details">
    <div class="dusk-balance__account">
      <span class="dusk-balance__percentage"
        ><Icon
          path={mdiShieldLock}
          data-tooltip-id="main-tooltip"
          data-tooltip-text="Shielded"
        />{numberFormatter(shieldedPercentage)}%</span
      >
      <span class="dusk-balance__value"
        >{duskFormatter(luxToDusk(shieldedAmount))}<Icon
          data-tooltip-id="main-tooltip"
          data-tooltip-text="DUSK"
          path={logo}
        /></span
      >
    </div>
    <div class="dusk-balance__account">
      <span class="dusk-balance__percentage"
        ><Icon
          path={mdiShieldLockOpenOutline}
          data-tooltip-id="main-tooltip"
          data-tooltip-text="Unshielded"
        />{numberFormatter(unshieldedPercentage)}%</span
      >
      <span class="dusk-balance__value"
        >{duskFormatter(luxToDusk(unshieldedAmount))}<Icon
          data-tooltip-id="main-tooltip"
          data-tooltip-text="DUSK"
          path={logo}
        /></span
      >
    </div>
  </Card>
</article>

<style lang="postcss">
  .dusk-balance__account {
    display: flex;
    justify-content: space-between;
    background-color: var(--background-color);
    padding: 0.625em 0.75em 0.625em 0.75em;
    border-radius: 1.5em;
  }

  .dusk-balance__percentage,
  .dusk-balance__value {
    display: flex;
    gap: var(--small-gap);
    align-items: center;
  }

  :global(.dusk-balance) {
    font-size: 1.25em;
    font-weight: 500;
    line-height: 1.5em;
  }

  .dusk-balance__fiat {
    margin-bottom: 1em;
  }
</style>
