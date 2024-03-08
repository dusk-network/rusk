<svelte:options immutable={true} />

<script>
  import { Icon, Tooltip } from "$lib/dusk/components";
  import { logo } from "$lib/dusk/icons";
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

  const duskFormatter = createCurrencyFormatter(locale, tokenCurrency, 9);
  const fiatFormatter = createCurrencyFormatter(locale, fiatCurrency, 2);

  $: classes = makeClassName(["dusk-balance", className]);
</script>

<article {...$$restProps} class={classes}>
  <header class="dusk-balance__header">
    <h2>Your Balance:</h2>
  </header>
  <p class="dusk-balance__dusk">
    <strong>{duskFormatter(tokens)}</strong>
    <Icon
      className="dusk-balance__icon"
      path={logo}
      data-tooltip-id="balance-tooltip"
      data-tooltip-text={tokenCurrency}
      data-tooltip-place="right"
    />
  </p>
  <p
    class="dusk-balance__fiat"
    class:dusk-balance__fiat--visible={fiatPrice !== undefined && tokens}
  >
    <strong>
      ({fiatFormatter(fiatPrice ? fiatPrice * tokens : 0)})
    </strong>
  </p>
  <Tooltip id="balance-tooltip" />
</article>
