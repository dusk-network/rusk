<svelte:options immutable={true} />

<script>
  import {
    DataCard,
    TransactionsList,
    TransactionsTable,
  } from "$lib/components";

  import "./TransactionsCard.css";

  /** @type {Transaction[] | null}*/
  export let txns;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  const ITEMS_TO_DISPLAY = 15;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  /** @type {number} */
  let clientWidth;

  /** @type {Transaction[]}*/
  let displayedTxns;

  /** @type {Boolean}*/
  let isLoadMoreDisabled = false;

  const loadMoreItems = () => {
    if (txns && itemsToDisplay < txns.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };

  $: displayedTxns = txns ? txns.slice(0, itemsToDisplay) : [];
  $: {
    if (txns && itemsToDisplay >= txns.length) {
      isLoadMoreDisabled = true;
    } else if (loading && txns === null) {
      isLoadMoreDisabled = true;
    } else {
      isLoadMoreDisabled = false;
    }
  }
</script>

<svelte:window bind:outerWidth={clientWidth} />
<DataCard
  on:retry
  data={txns}
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
      data={displayedTxns}
      className="transactions-card__table"
    />
  {:else}
    <div class="transactions-card__list">
      {#each displayedTxns as txn (txn)}
        <TransactionsList data={txn} />
      {/each}
    </div>
  {/if}
</DataCard>
