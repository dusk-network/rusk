<svelte:options immutable={true} />

<script>
  import { Rerender } from "../..";

  /** @type {number | undefined} */
  export let interval = undefined;

  let counter = 0;
</script>

<Rerender {interval}>{counter++}</Rerender>
