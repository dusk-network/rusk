<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";

  import "./Icon.css";

  /** @type {String|Undefined} */
  export let className = undefined;

  /** @type {IconSize} */
  export let size = "normal";

  /** @type {Boolean} */
  export let isInStack = false;

  /** @type {String} */
  export let path;

  /** @type {String} */
  export let role = "graphics-symbol";

  $: commonAttributes = {
    ...$$restProps,
    class: makeClassName(["dusk-icon", `dusk-icon--size--${size}`, className]),
  };
</script>

{#if isInStack}
  <g {...commonAttributes}>
    <rect />
    <path d={path} />
  </g>
{:else}
  <svg {...commonAttributes} {role} viewBox="0 0 24 24">
    <path d={path} />
  </svg>
{/if}
