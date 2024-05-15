<svelte:options immutable={true} />

<script>
  import {
    DataCard,
    TransactionsList,
    TransactionsTable,
  } from "$lib/components";

  import "./TransactionsCard.css";

  /** @type {Transaction[] | null}*/
  export let txs;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  const ITEMS_TO_DISPLAY = 15;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  /** @type {number} */
  let clientWidth;

  /** @type {Transaction[]}*/
  let displayedTxs;

  /** @type {Boolean}*/
  let isLoadMoreDisabled = false;

  const loadMoreItems = () => {
    if (txs && itemsToDisplay < txs.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };

  $: displayedTxs = txs ? txs.slice(0, itemsToDisplay) : [];
  $: {
    if (txs && itemsToDisplay >= txs.length) {
      isLoadMoreDisabled = true;
    } else if (loading && txs === null) {
      isLoadMoreDisabled = true;
    } else {
      isLoadMoreDisabled = false;
    }
  }
</script>

<svelte:window bind:outerWidth={clientWidth} />
<DataCard
  on:retry
  data={txs}
  {error}
  {loading}
  title="Transactions"
  headerButtonDetails={{
    action: () => loadMoreItems(),
    disabled: isLoadMoreDisabled,
    label: "Show More",
  }}
>
  {#if clientWidth > 768}
    <TransactionsTable
      data={displayedTxs}
      className="transactions-card__table"
    />
  {:else}
    <div class="transactions-card__list">
      {#each displayedTxs as tx (tx)}
        <TransactionsList data={tx} />
      {/each}
    </div>
  {/if}
</DataCard>
