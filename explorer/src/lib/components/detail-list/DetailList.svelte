<svelte:options immutable={true} />

<script>
  import { setContext } from "svelte";
  import contexts from "$lib/contexts";
  import { makeClassName } from "$lib/dusk/string";
  import "./DetailList.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  setContext("DUSK:list-item:context", contexts.LIST_ITEM.DETAIL_LIST);

  $: classes = makeClassName(["detail-list", className]);
</script>

<dl class={classes}>
  <slot />
</dl>
