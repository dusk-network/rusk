<script>
  import { ProgressBar } from "$lib/dusk/components";

  /** @type {bigint} */
  export let from;
  /** @type {bigint} */
  export let last;
  /** @type {number} */
  export let progress;
</script>

<div class="sync-bar">
  <span>{from.toLocaleString()}</span><ProgressBar
    currentPercentage={progress * 100}
  /><span>{last.toLocaleString()}</span>
</div>

<style>
  .sync-bar {
    display: flex;
    flex-direction: row;
    align-items: baseline;
    gap: var(--small-gap);
    width: 100%;
  }
</style>
