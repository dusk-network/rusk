<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";
  import "./ProgressBar.css";

  /** @type {number | undefined} */
  export let currentPercentage = undefined;

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {string} */
  export let ariaLabel;

  $: classes = makeClassName(["dusk-progress-bar", className]);
</script>

<div role="progressbar" class={classes} aria-label={ariaLabel}>
  <div
    style={currentPercentage !== undefined
      ? `width: ${currentPercentage}%`
      : undefined}
    class:dusk-progress-bar__filler--undetermined={currentPercentage ===
      undefined}
    class="dusk-progress-bar__filler"
  ></div>
</div>
