<script>
  /** @type {"td" | "th"} */
  export let type = "td";
</script>

<svelte:element
  this={type}
  class={type === "td" ? "table__data-cell" : "table__header-cell"}
>
  <slot />
</svelte:element>
