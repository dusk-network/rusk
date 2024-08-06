<svelte:options immutable={true} />

<script>
  import { BlocksList, BlocksTable, DataCard } from "$lib/components";

  import "./BlocksCard.css";

  /** @type {Block[] | null}*/
  export let blocks;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {AppStore} */
  export let appStore;

  const ITEMS_TO_DISPLAY = 15;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  /** @type {Block[]}*/
  let displayedBlocks;

  $: displayedBlocks = blocks ? blocks.slice(0, itemsToDisplay) : [];
  $: isLoadMoreDisabled =
    (blocks && itemsToDisplay >= blocks.length) || (loading && blocks === null);
  $: ({ isSmallScreen } = $appStore);

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
  headerButtonDetails={{
    action: () => loadMoreItems(),
    disabled: isLoadMoreDisabled,
    label: "Show More",
  }}
>
  {#if isSmallScreen}
    <div class="blocks-card__list">
      {#each displayedBlocks as block (block)}
        <BlocksList data={block} />
      {/each}
    </div>
  {:else}
    <BlocksTable data={displayedBlocks} className="blocks-card__table" />
  {/if}
</DataCard>
