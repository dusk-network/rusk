<script>
  import { makeClassName } from "$lib/dusk/string";

  /** @type {string} */
  export let tag = "div";

  /** @type {CardGap} */
  export let gap = "default";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {boolean} */
  export let onSurface = false;

  $: classes = makeClassName(["dusk-card", `dusk-card--gap-${gap}`, className]);
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
