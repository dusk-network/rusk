<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";
  import "./Anchor.css";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {String} */
  export let href;

  /** @type {boolean} */
  export let onSurface = true;
</script>

<a
  {...$$restProps}
  class={makeClassName([
    "dusk-anchor",
    `dusk-anchor--${onSurface ? "on-surface" : "off-surface"}`,
    className,
  ])}
  {href}
  on:click
>
  <slot />
</a>
