<svelte:options immutable={true} />

<script>
  import {
    DataCard,
    TransactionsList,
    TransactionsTable,
  } from "$lib/components";

  /** @type {Transaction[] | null}*/
  export let txns;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {boolean} */
  export let isSmallScreen;

  const ITEMS_TO_DISPLAY = import.meta.env.VITE_CHAIN_INFO_ENTRIES;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  /** @type {Transaction[]}*/
  let displayedTxns;

  $: displayedTxns = txns ? txns.slice(0, itemsToDisplay) : [];
  $: isLoadMoreDisabled =
    (txns && itemsToDisplay >= txns.length) || (loading && txns === null);

  const loadMoreItems = () => {
    if (txns && itemsToDisplay < txns.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };
</script>

<DataCard
  on:retry
  data={txns}
  {error}
  {loading}
  title="Transactions — {displayedTxns.length} Displayed Items"
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
      {#each displayedTxns as txn (txn)}
        <TransactionsList data={txn} mode="full" />
      {/each}
    </div>
  {:else}
    <TransactionsTable
      data={displayedTxns}
      displayTooltips={true}
      mode="full"
    />
  {/if}
</DataCard>
