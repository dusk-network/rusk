<script>
  import { slide } from "svelte/transition";
  import { Button } from "$lib/dusk/components";
  import { GasControls, GasFee } from "$lib/components";
  import { areValidGasSettings } from "$lib/contracts";
  import { onMount } from "svelte";

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

  /** @type {bigint} */
  export let fee;

  /** @type {(amount: number) => string} */
  export let formatter;

  /** @type {boolean} */
  let isExpanded = false;

  onMount(() => {
    if (!areValidGasSettings(price, limit)) {
      isExpanded = true;
    }
  });
</script>

<div class="gas-settings">
  <dl class="gas-settings__edit">
    <GasFee {formatter} {fee} />
    <dd>
      <Button
        size="small"
        variant="tertiary"
        on:click={() => {
          isExpanded = !isExpanded;
        }}
        text={isExpanded ? "CLOSE" : "EDIT"}
      />
    </dd>
  </dl>
  {#if isExpanded}
    <div in:slide|global class="gas-settings">
      <GasControls
        on:gasSettings
        {limit}
        {limitLower}
        {limitUpper}
        {price}
        {priceLower}
      />
    </div>
  {/if}
</div>

<style lang="postcss">
  .gas-settings {
    display: flex;
    flex-direction: column;
    gap: 0.625em;
  }

  .gas-settings__edit {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
  }
</style>
