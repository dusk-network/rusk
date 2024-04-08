<svelte:options immutable={true} />

<script>
  import { addBasePath } from "$lib/navigation";

  /** @type {string} */
  export let alt;

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {string} */
  export let src;
</script>

<img {...$$restProps} {alt} class={className} src={addBasePath(src)} />
