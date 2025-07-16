<script>
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { duskIcon } from "$lib/dusk/icons";
  import { appStore } from "$lib/stores";
  import {
    DataCard,
    TokenDetailsTable,
    TokenOverview,
    TokenTransactionsList,
  } from "$lib/components";
  import { gqlTokenTransactions, tokens } from "$lib/mock-data";

  const ITEMS_TO_DISPLAY = import.meta.env.VITE_CHAIN_INFO_ENTRIES;
  let itemsToDisplay = ITEMS_TO_DISPLAY;

  const url = new URL($page.url);
  const tokenName = url.searchParams.get("name");
  const tokenData = tokens.find((token) => token.token === tokenName);

  const transactions = gqlTokenTransactions.map((transaction) => {
    return {
      ...transaction,
      blobHashes: [
        "3656d71948baff2091090423f3b07701223b00d1a10942e44afe644a30865423",
        "d26d6ebba9bfb0504040eadec111627f9f562c358f40e035ea9011b48ed7b55b",
      ],
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
  {#if tokenData}
    <article>
      <TokenOverview {screenWidth} iconPath={duskIcon} data={tokenData} />
    </article>
    <DataCard
      on:retry
      data={transactions}
      {error}
      {loading}
      title="{tokenData?.token.toUpperCase()} Transactions — {displayedTxns.length} Displayed Items"
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
    <p>Token not found</p>
  {/if}
</section>
