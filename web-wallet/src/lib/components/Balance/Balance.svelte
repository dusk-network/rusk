<svelte:options immutable={true} />

<script>
  import { UsageIndicator } from "$lib/components";
  import { makeClassName } from "$lib/dusk/string";
  import { createCurrencyFormatter } from "$lib/dusk/currency";

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

  /** @type {number} */
  export let tokens;

  /** @type {number | undefined} */
  export let shieldedTokensPercentage = undefined;

  $: classes = makeClassName(["dusk-balance", className]);
  $: duskFormatter = createCurrencyFormatter(locale, tokenCurrency, 9);
  $: fiatFormatter = createCurrencyFormatter(locale, fiatCurrency, 2);
</script>

<article {...$$restProps} class={classes}>
  <header class="dusk-balance__header">
    <h2 class="sr-only">Your Balance:</h2>
  </header>
  <p class="dusk-balance__dusk">
    <strong>{duskFormatter(tokens)}</strong>
    <strong>{tokenCurrency}</strong>
  </p>
  <p
    class="dusk-balance__fiat"
    class:dusk-balance__fiat--visible={fiatPrice !== undefined && tokens}
  >
    <strong>
      {fiatFormatter(fiatPrice ? fiatPrice * tokens : 0)}
    </strong>
  </p>
  {#if shieldedTokensPercentage}
    <UsageIndicator
      className="dusk-balance__usage"
      value={shieldedTokensPercentage}
    />
  {/if}
</article>
