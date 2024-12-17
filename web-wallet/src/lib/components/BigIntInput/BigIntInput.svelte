<svelte:options immutable />

<script>
  import { createEventDispatcher, onMount } from "svelte";
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

  let internalValue = value.toString();

  /** @type {(v: bigint, min: bigint, max: bigint) => boolean} */
  const isInvalidInput = (v, min, max) => !!(min > v || v > max);

  const checkValidity = () => {
    if (isInvalidInput(value, minValue, maxValue)) {
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

  onMount(() => {
    checkValidity();
  });

  $: inputClass = makeClassName({
    "invalid-input": isInvalidInput(value, minValue, maxValue),
    [`${className}`]: true,
  });
  $: {
    internalValue = value.toString();
  }
</script>

<Textbox
  {...$$restProps}
  className={inputClass}
  inputmode="numeric"
  type="text"
  bind:value={internalValue}
  on:input={validateInput}
  on:input
  pattern="\d+"
/>
