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

  const ITEMS_TO_DISPLAY = 15;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  /** @type {number} */
  let clientWidth;

  /** @type {Block[]}*/
  let displayedBlocks;

  /** @type {Boolean}*/
  let isLoadMoreDisabled = false;

  const loadMoreItems = () => {
    if (blocks && itemsToDisplay < blocks.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };

  $: displayedBlocks = blocks ? blocks.slice(0, itemsToDisplay) : [];
  $: {
    if (blocks && itemsToDisplay >= blocks.length) {
      isLoadMoreDisabled = true;
    } else if (loading && blocks === null) {
      isLoadMoreDisabled = true;
    } else {
      isLoadMoreDisabled = false;
    }
  }
</script>

<svelte:window bind:outerWidth={clientWidth} />
<DataCard
  on:retry
  data={blocks}
  {error}
  {loading}
  title="Blocks"
  headerButtonDetails={{
    action: () => loadMoreItems(),
    disabled: isLoadMoreDisabled,
    label: "Show More",
  }}
>
  {#if clientWidth > 768}
    <BlocksTable data={displayedBlocks} className="blocks-card__table" />
  {:else}
    <div class="blocks-card__list">
      {#each displayedBlocks as block (block)}
        <BlocksList data={block} />
      {/each}
    </div>
  {/if}
</DataCard>
