<svelte:options immutable={true} />

<script>
  import { BlocksList, BlocksTable, DataCard } from "$lib/components";
  import { makeClassName } from "$lib/dusk/string";
  import { goto } from "$lib/navigation";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Block[]}*/
  export let blocks;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {boolean} */
  export let isSmallScreen;

  $: classes = makeClassName(["latest-blocks-card", className]);
</script>

<DataCard
  on:retry
  data={blocks}
  {error}
  {loading}
  className={classes}
  title="Blocks"
  headerButtonDetails={{
    action: () => goto("/blocks"),
    disabled: false,
    label: "All Blocks",
  }}
>
  {#if isSmallScreen}
    {#each blocks as block (block)}
      <BlocksList data={block} />
    {/each}
  {:else}
    <BlocksTable data={blocks} />
  {/if}
</DataCard>
