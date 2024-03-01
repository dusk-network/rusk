<svelte:options immutable={true} />

<script>
  import { createEventDispatcher } from "svelte";

  import { makeClassName } from "$lib/dusk/string";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {Boolean} */
  export let disabled = false;

  /** @type {Number} */
  export let tabindex = 0;

  export let onSurface = false;

  /** @type {Boolean} */
  export let value = false;

  const dispatch = createEventDispatcher();

  /** @type {import("svelte/elements").MouseEventHandler<HTMLDivElement>} */
  function handleClick() {
    if (!disabled) {
      toggleSwitch();
    }
  }

  /** @type {import("svelte/elements").KeyboardEventHandler<HTMLDivElement>} */
  function handleKeyDown(event) {
    if (!disabled && event.key === " ") {
      toggleSwitch();
    }
  }

  function toggleSwitch() {
    value = !value;

    dispatch("change", value);
  }
</script>

<div
  {...$$restProps}
  aria-checked={value}
  aria-disabled={disabled}
  class={makeClassName(["dusk-switch", className])}
  class:dusk-switch--on-surface={onSurface}
  on:click={handleClick}
  on:keydown|preventDefault={handleKeyDown}
  role="switch"
  tabindex={disabled ? -1 : tabindex}
/>
