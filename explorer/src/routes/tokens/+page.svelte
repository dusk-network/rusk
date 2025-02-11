<script>
  import { DataCard, TokenListDetails, TokensTable } from "$lib/components";
  import { appStore } from "$lib/stores";
  import { tokens } from "$lib/mock-data";

  const ITEMS_TO_DISPLAY = import.meta.env.VITE_CHAIN_INFO_ENTRIES;
  let itemsToDisplay = ITEMS_TO_DISPLAY;

  const error = null;
  const loading = false;

  const loadMoreItems = () => {
    if (tokens && itemsToDisplay < tokens.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };

  $: ({ isSmallScreen } = $appStore);

  $: displayedTokens = tokens ? tokens.slice(0, itemsToDisplay) : [];
  $: isLoadMoreDisabled =
    (tokens && itemsToDisplay >= tokens.length) || (loading && tokens === null);
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
