<svelte:options immutable={true} />

<script>
  import { BlocksList, BlocksTable, DataCard } from "$lib/components";

  /** @type {Block[] | null}*/
  export let blocks;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {boolean} */
  export let isSmallScreen;

  const ITEMS_TO_DISPLAY = import.meta.env.VITE_CHAIN_INFO_ENTRIES;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  /** @type {Block[]}*/
  let displayedBlocks;

  $: displayedBlocks = blocks ? blocks.slice(0, itemsToDisplay) : [];
  $: isLoadMoreDisabled =
    (blocks && itemsToDisplay >= blocks.length) || (loading && blocks === null);

  const loadMoreItems = () => {
    if (blocks && itemsToDisplay < blocks.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };
</script>

<DataCard
  on:retry
  data={blocks}
  {error}
  {loading}
  title="Blocks — {displayedBlocks.length} Displayed Items"
  headerButtonDetails={error
    ? undefined
    : {
        action: () => loadMoreItems(),
        disabled: isLoadMoreDisabled,
        label: "Show More",
      }}
>
  {#if isSmallScreen}
    <div class="data-card__list">
      {#each displayedBlocks as block (block)}
        <BlocksList data={block} />
      {/each}
    </div>
  {:else}
    <BlocksTable data={displayedBlocks} className="data-card__table" />
  {/if}
</DataCard>
