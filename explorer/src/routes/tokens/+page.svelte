<script>
  import { DataCard, TokensTable, TokenListDetails } from "$lib/components";
  import { appStore } from "$lib/stores";
  import { tokens } from "$lib/mock-data";

  const ITEMS_TO_DISPLAY = import.meta.env.VITE_CHAIN_INFO_ENTRIES;

  $: ({ isSmallScreen } = $appStore);

  let error = null;
  let loading = false;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  $: displayedTokens = tokens ? tokens.slice(0, itemsToDisplay) : [];
  $: isLoadMoreDisabled =
    (tokens && itemsToDisplay >= tokens.length) || (loading && tokens === null);

  const loadMoreItems = () => {
    if (tokens && itemsToDisplay < tokens.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };
</script>

<section>
  <DataCard
    on:retry
    data={displayedTokens}
    {error}
    {loading}
    title="Tokens — {displayedTokens.length} Displayed Items"
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
        {#each displayedTokens as token (token)}
          <TokenListDetails data={token} />
        {/each}
      </div>
    {:else}
      <TokensTable data={tokens} />
    {/if}
  </DataCard>
</section>
