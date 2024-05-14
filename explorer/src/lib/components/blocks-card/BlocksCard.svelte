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

  /** @type {number} */
  let clientWidth;

  $: classes = makeClassName(["blocks-card", className]);
</script>

<svelte:window bind:outerWidth={clientWidth} />
<DataCard
  on:retry
  data={blocks}
  {error}
  {loading}
  className={classes}
  title="Blocks"
  headerButtonDetails={{ action: () => goto("/blocks"), label: "All Blocks" }}
>
  {#if clientWidth > 768}
    <BlocksTable data={blocks} />
  {:else}
    {#each blocks as block (block)}
      <BlocksList data={block} />
    {/each}
  {/if}
</DataCard>
