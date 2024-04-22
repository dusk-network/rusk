<script>
  import { makeClassName } from "$lib/dusk/string";

  import "./Card.css";

  /** @type {string} */
  export let tag = "div";

  /** @type {string | Undefined} */
  export let className = undefined;

  export let onSurface = false;

  $: classes = makeClassName(["dusk-card", className]);
</script>

<svelte:element
  this={tag}
  {...$$restProps}
  class={classes}
  class:dusk-card--on-surface={onSurface}
>
  {#if $$slots.header}
    <slot name="header" />
  {/if}
  <slot />
  {#if $$slots.footer}
    <slot name="footer" />
  {/if}
</svelte:element>
