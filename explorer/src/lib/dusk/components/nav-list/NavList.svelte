<script>
  import { makeClassName } from "$lib/dusk/string";
  import { AppAnchor } from "$lib/components";
  import "./NavList.css";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type { NavListProp } */
  export let navigation;

  $: classes = makeClassName(["dusk-nav-list", className]);
</script>

{#if navigation}
  <nav class={classes}>
    {#each navigation as item, index (`${item.title}-${index}`)}
      <AppAnchor className="dusk-nav-list__link" href={item.link}
        >{item.title}</AppAnchor
      >
    {/each}
  </nav>
{/if}
