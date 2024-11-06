<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { BigIntInput } from "$lib/components";

  /** @type {bigint} */
  export let limit;

  /** @type {bigint} */
  export let limitLower;

  /** @type {bigint} */
  export let limitUpper;

  /** @type {bigint} */
  export let price;

  /** @type {bigint} */
  export let priceLower;

  const dispatch = createEventDispatcher();

  function dispatchGasLimits() {
    dispatch("gasSettings", {
      limit: limit,
      price: price,
    });
  }

  onMount(() => {
    dispatchGasLimits();
  });
</script>

<label for={undefined} class="gas-control">
  <span class="gas-control__label"> Price (lux):</span>
  <BigIntInput
    bind:value={price}
    className="gas-control__input"
    maxValue={limit}
    minValue={priceLower}
    on:input={dispatchGasLimits}
    required
  />
</label>

<label for={undefined} class="gas-control">
  <span class="gas-control__label"> Gas Limit (unit):</span>
  <BigIntInput
    bind:value={limit}
    className="gas-control__input"
    maxValue={limitUpper}
    minValue={limitLower}
    on:input={dispatchGasLimits}
    required
  />
</label>

<style lang="postcss">
  .gas-control {
    display: flex;
    gap: 0.5em;
    width: 100%;
    flex-direction: column;
    justify-content: start;
    align-items: stretch;

    &__label {
      line-height: 140%;
    }
  }
</style>
