<svelte:options immutable />

<script>
  import { createEventDispatcher } from "svelte";
  import { Textbox } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";
  import "./BigIntInput.css";

  /** @type {string | undefined} */
  export let className = undefined;

  const dispatch = createEventDispatcher();

  /** @type {bigint} */
  export let minValue = 0n;

  /** @type {bigint} */
  export let maxValue = 9007199254740991n;

  /** @type {bigint} */
  export let value = 0n;

  let isInvalidInput = false;
  let internalValue = value.toString();

  const checkValidity = () => {
    isInvalidInput = !!(minValue > value || value > maxValue);
    if (isInvalidInput) {
      dispatch("error", "Value exceeds limits");
    }
  };

  function validateInput() {
    try {
      value = BigInt(internalValue);
      checkValidity();
      dispatch("change", value);
    } catch (error) {
      internalValue = value.toString();
    }
  }

  $: inputClass = makeClassName({
    "invalid-input": isInvalidInput,
    [`${className}`]: true,
  });
</script>

<Textbox
  className={inputClass}
  inputmode="numeric"
  type="text"
  bind:value={internalValue}
  on:input={validateInput}
  pattern="\d+"
/>
