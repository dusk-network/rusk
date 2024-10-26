<svelte:options immutable />

<script>
  import { createEventDispatcher } from "svelte";
  import { Textbox } from "$lib/dusk/components";

  /** @type {string | undefined} */
  export let className = undefined;

  const dispatch = createEventDispatcher();

  /** @type {bigint} */
  export let value = 0n;

  let internalValue = value.toString();

  function validateInput() {
    try {
      value = BigInt(internalValue);
      dispatch("update", value);
    } catch {
      internalValue = value.toString();
    }
  }
</script>

<Textbox
  {className}
  type="text"
  bind:value={internalValue}
  on:input={validateInput}
  pattern="\d+"
/>
