<svelte:options immutable={true} />

<script>
  import {
    DataCard,
    ProvisionersList,
    ProvisionersTable,
  } from "$lib/components";

  /** @type {HostProvisioner[] | null}*/
  export let provisioners;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {boolean} */
  export let isSmallScreen;

  const ITEMS_TO_DISPLAY = import.meta.env.VITE_CHAIN_INFO_ENTRIES;

  let itemsToDisplay = ITEMS_TO_DISPLAY;

  /** @type {HostProvisioner[]}*/
  let displayedProvisioner;

  $: displayedProvisioner = provisioners
    ? provisioners.slice(0, itemsToDisplay)
    : [];
  $: isLoadMoreDisabled =
    (provisioners && itemsToDisplay >= provisioners.length) ||
    (loading && provisioners === null);

  const loadMoreItems = () => {
    if (provisioners && itemsToDisplay < provisioners.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };
</script>

<DataCard
  on:retry
  data={provisioners}
  {error}
  {loading}
  title="Provisioners — {displayedProvisioner.length} Displayed Items"
  headerButtonDetails={error
    ? undefined
    : {
        action: () => loadMoreItems(),
        disabled: isLoadMoreDisabled,
        label: "Show More",
      }}
>
  {#if isSmallScreen}
    <div class="provisioners-card__list">
      {#each displayedProvisioner as provisioner (provisioner)}
        <ProvisionersList data={provisioner} displayTooltips={true} />
      {/each}
    </div>
  {:else}
    <ProvisionersTable
      data={displayedProvisioner}
      className="provisioners-card__table"
    />
  {/if}
</DataCard>
