<svelte:options immutable={true} />

<script>
  import { RelativeTime } from "../..";

  /** @type {Date} */
  export let date;
</script>

<RelativeTime autoRefresh={true} {date}>
  <svelte:fragment let:relativeTime>
    <span>The relative time now is {relativeTime}</span>
  </svelte:fragment>
</RelativeTime>
