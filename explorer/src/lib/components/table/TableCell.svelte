<script>
  import { makeClassName } from "$lib/dusk/string";

  /** @type {"td" | "th"} */
  export let type = "td";

  /** @type {"left" | "center" | "right" | Undefined} */
  export let align = "left";
</script>

<svelte:element
  this={type}
  class={makeClassName([
    type === "td" ? "table__data-cell" : "table__header-cell",
    align ? `align-${align}` : "",
  ])}
>
  <slot />
</svelte:element>
