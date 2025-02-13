<script>
  import { onMount } from "svelte";
  import { appStore } from "$lib/stores";
  import {
    AccountOverview,
    DataCard,
    TokenDetailsTable,
    TokenTransactionsList,
  } from "$lib/components";
  import { accounts, gqlTokenTransactions } from "$lib/mock-data";

  const ITEMS_TO_DISPLAY = import.meta.env.VITE_CHAIN_INFO_ENTRIES;
  let itemsToDisplay = ITEMS_TO_DISPLAY;

  const accountData = accounts[0];

  const transactions = gqlTokenTransactions.map((transaction) => {
    return {
      ...transaction,
      date: new Date(transaction.date),
    };
  });

  const error = null;
  const loading = false;

  const loadMoreItems = () => {
    if (transactions && itemsToDisplay < transactions.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };

  $: ({ isSmallScreen } = $appStore);

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {Transaction[]}*/
  let displayedTxns;
  $: displayedTxns = transactions ? transactions.slice(0, itemsToDisplay) : [];

  $: isLoadMoreDisabled =
    (transactions && itemsToDisplay >= transactions.length) ||
    (loading && transactions === null);

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });
</script>

<section>
  {#if accountData}
    <article>
      <AccountOverview {screenWidth} data={accountData} />
    </article>
    <DataCard
      on:retry
      data={transactions}
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
            <TokenTransactionsList data={txn} />
          {/each}
        </div>
      {:else}
        <TokenDetailsTable data={displayedTxns} />
      {/if}
    </DataCard>
  {:else}
    <p>Account data not found</p>
  {/if}
</section>
