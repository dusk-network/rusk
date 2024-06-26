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

<DataCard
  on:retry
  data={txns}
  {error}
  {loading}
  title="Transactions  - {displayedTxns.length} Displayed Items"
  headerButtonDetails={{
    action: () => loadMoreItems(),
    disabled: isLoadMoreDisabled,
    label: "Show More",
  }}
>
  <TransactionsTable
    className="transactions-card__table mobile-hidden"
    data={displayedTxns}
    mode="full"
  />

  <div class="transactions-card__list desktop-hidden">
    {#each displayedTxns as txn (txn)}
      <TransactionsList data={txn} mode="full" />
    {/each}
  </div>
</DataCard>
