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
    <div class="dusk-card__header-container">
      <slot name="header" />
    </div>
  {/if}
  <div class="dusk-card__body-container">
    <slot />
  </div>
  {#if $$slots.footer}
    <div class="dusk-card__footer-container">
      <slot name="footer" />
    </div>
  {/if}
</svelte:element>
