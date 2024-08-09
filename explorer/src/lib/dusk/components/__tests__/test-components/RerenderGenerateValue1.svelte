<svelte:options immutable={true} />

<script>
  import { Rerender } from "../..";

  /** @type {Array<any>} */
  export let values;

  let index = 0;

  /** @type {() => number}*/
  const generateValue = () => values[index++];
</script>

<Rerender {generateValue}></Rerender>
