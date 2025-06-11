<svelte:options immutable={true} />

<script>
  import { tweened } from "svelte/motion";
  import { expoOut } from "svelte/easing";
  import { makeClassName } from "$lib/dusk/string";

  /** @type {number|undefined} */
  export let currentPercentage = undefined;

  /** @type {string|undefined} */
  export let className = undefined;

  /** @type {number} */
  export let motionDuration = 400;

  $: classes = makeClassName(["dusk-progress-bar", className]);

  const progress = tweened(0, {
    duration: motionDuration,
    easing: expoOut,
  });

  $: currentPercentage !== undefined && progress.set(currentPercentage);
</script>

<div role="progressbar" class={classes}>
  <div
    style={currentPercentage !== undefined ? `width: ${$progress}%` : undefined}
    class:dusk-progress-bar__filler--undetermined={currentPercentage ===
      undefined}
    class="dusk-progress-bar__filler"
  />
</div>
