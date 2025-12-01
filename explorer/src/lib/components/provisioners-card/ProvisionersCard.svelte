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

  /**
   * @typedef {HostProvisioner & {
   *   rank: number;
   *   ownerType: string;
   *   ownerAddress: string;
   *   hasSeparateOwner: boolean;
   * }} EnrichedProvisioner
   */

  /** @param {HostProvisioner} p */
  const hasActiveStake = (p) =>
    p.amount !== 0 || p.locked_amt !== 0 || p.eligibility !== 0;

  /**
   * Reduce callback, filters inactive + enriches owner fields.
   * @param {EnrichedProvisioner[]} acc
   * @param {HostProvisioner} p
   * @returns {EnrichedProvisioner[]}
   */
  const toEnrichedProvisioner = (acc, p) => {
    if (!hasActiveStake(p)) return acc;

    const [ownerType, ownerAddress] = Object.entries(p.owner ?? {})[0] ?? [
      "",
      "",
    ];

    const hasSeparateOwner = Boolean(ownerAddress) && ownerAddress !== p.key;

    acc.push({
      ...p,
      hasSeparateOwner,
      ownerAddress,
      ownerType,
      rank: 0,
    });

    return acc;
  };

  /** @type {EnrichedProvisioner[]} */
  $: enrichedProvisioners = Array.isArray(provisioners)
    ? provisioners
        .reduce(
          toEnrichedProvisioner,
          /** @type {EnrichedProvisioner[]} */ ([])
        )
        .toSorted((a, b) => Number(b.amount ?? 0) - Number(a.amount ?? 0))
        .map((p, index) => ({
          ...p,
          rank: index + 1,
        }))
    : [];

  $: displayedProvisioners = enrichedProvisioners.slice(0, itemsToDisplay);
  $: isLoadMoreDisabled =
    itemsToDisplay >= enrichedProvisioners.length || (loading && !provisioners);

  const loadMoreItems = () => {
    if (itemsToDisplay < enrichedProvisioners.length) {
      itemsToDisplay += ITEMS_TO_DISPLAY;
    }
  };
</script>

<DataCard
  on:retry
  data={provisioners}
  {error}
  {loading}
  title="Provisioners — {displayedProvisioners.length} Displayed Items"
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
      {#each displayedProvisioners as provisioner (provisioner)}
        <ProvisionersList data={provisioner} displayTooltips={true} />
      {/each}
    </div>
  {:else}
    <ProvisionersTable
      data={displayedProvisioners}
      className="data-card__table"
    />
  {/if}
</DataCard>
