<svelte:options immutable={true} />

<script>
  import { AnchorButton } from "$lib/dusk/components";
  import { addBasePath } from "$lib/navigation";

  /** @type {string} */
  export let href;
</script>

<AnchorButton {...$$restProps} href={addBasePath(href)} on:click />
