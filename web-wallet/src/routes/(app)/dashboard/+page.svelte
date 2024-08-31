<svelte:options immutable={true} />

<script>
  import { filterWith, hasKeyValue } from "lamb";
  import {
    mdiContain,
    mdiDatabaseOutline,
    mdiSwapVertical,
    mdiSync,
  } from "@mdi/js";
  import { Icon } from "$lib/dusk/components";
  import { DashboardNav, Transactions } from "$lib/components";
  import { settingsStore, walletStore } from "$lib/stores";
  import { contractDescriptors } from "$lib/contracts";

  const { dashboardTransactionLimit, language } = $settingsStore;

  /**
   * @param {string} contract
   */
  function getIconsForContract(contract) {
    /** @type {Array.<DashboardNavItemIconProp>} */
    let icons = [{ path: "" }];

    switch (contract) {
      case "allocate":
        icons = [{ path: mdiSync }];
        break;
      case "staking":
        icons = [{ path: mdiDatabaseOutline }];
        break;
      case "transfer":
        icons = [{ path: mdiSwapVertical }];
        break;
      default:
        break;
    }

    return icons;
  }

  /** @type {(descriptors: ContractDescriptor[]) => ContractDescriptor[]} */
  const getEnabledContracts = filterWith(hasKeyValue("disabled", false));
  const enabledContracts = getEnabledContracts(contractDescriptors);
  const dashboardNavItems = enabledContracts.map(({ id, label }) => ({
    href: id,
    icons: getIconsForContract(id),
    id,
    label,
  }));
  const hasNoEnabledContracts = enabledContracts.length === 0;

  $: ({ isSyncing, error } = $walletStore);
</script>

{#if hasNoEnabledContracts}
  <div class="no-contracts">
    <Icon path={mdiContain} size="large" />
    <h3>No Contracts Enabled</h3>
    <p>
      It appears that no contracts are currently enabled. To access the full
      range of functionalities, enabling contracts is essential.
    </p>
    {#if import.meta.env.MODE === "development"}
      <h4>For Developers:</h4>
      <p>No contracts are currently enabled.</p>
    {/if}
  </div>
{/if}

<DashboardNav items={dashboardNavItems} />

<slot />

<Transactions
  items={walletStore.getTransactionsHistory()}
  {language}
  limit={dashboardTransactionLimit}
  {isSyncing}
  syncError={error}
/>

<style lang="postcss">
  .no-contracts {
    display: flex;
    flex-direction: column;
    background-color: var(--surface-color);
    padding: 1rem 1.375rem;
    border-radius: var(--control-border-radius-size);

    & h3 {
      text-align: center;
      margin-bottom: 1em;
    }

    & p:not(:last-child) {
      margin-bottom: 1em;
    }
    h4 {
      margin-bottom: 0.5em;
    }

    :global(.dusk-icon) {
      align-self: center;
      margin-bottom: 0.5rem;
    }
  }
</style>
