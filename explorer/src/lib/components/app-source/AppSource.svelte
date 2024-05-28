<svelte:options immutable={true} />

<script>
  import { joinWith, mapWith, pipe, replace, splitBy } from "lamb";

  import { addBasePath } from "$lib/navigation";

  /** @type {string | undefined} */
  export let src = undefined;

  /** @type {string | undefined} */
  export let srcset = undefined;

  const addBasePathToSrcSet = pipe([
    splitBy(","),
    mapWith(replace(/[^\s]+(?=\s)?/, addBasePath)),
    joinWith(","),
  ]);
</script>

<source
  {...$$restProps}
  src={src ? addBasePath(src) : undefined}
  srcset={srcset ? addBasePathToSrcSet(srcset) : undefined}
/>
