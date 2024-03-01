<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";

  import "./Icon.css";

  /** @type {String|Undefined} */
  export let className = undefined;

  /** @type {String|Undefined} */
  export let bgColor = undefined;

  /** @type {String|Undefined} */
  export let color = undefined;

  $: commonAttributes = {
    ...$$restProps,
    class: makeClassName(["dusk-icon--circular", className]),
  };
</script>

<div
  style={`background-color: ${bgColor ? `${bgColor}` : "var(--secondary-color)"};
			 color: ${color ? `${color}` : "var(--on-secondary-color)"}`}
  {...commonAttributes}
>
  <slot />
</div>
