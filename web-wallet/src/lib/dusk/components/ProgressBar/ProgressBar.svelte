<script>
  /** @type {number | undefined} */
  export let currentPercentage = undefined;
</script>

<div role="progressbar" class="dusk-progress-bar">
  <div
    style={currentPercentage !== undefined
      ? `width: ${currentPercentage}%`
      : undefined}
    class:dusk-progress-bar__filler--undetermined={currentPercentage ===
      undefined}
    class="dusk-progress-bar__filler"
  />
</div>
