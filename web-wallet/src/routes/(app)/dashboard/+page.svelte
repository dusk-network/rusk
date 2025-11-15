<svelte:options immutable={true} />

<script>
  import {
    mdiArrowBottomLeft,
    mdiArrowTopRight,
    mdiBridge,
    mdiContain,
    mdiDatabaseOutline,
    mdiListBoxOutline,
    mdiSwapHorizontal,
    mdiSync,
  } from "@mdi/js";
  import { AnchorButton, Icon } from "$lib/dusk/components";
  import { DashboardNav, Transactions } from "$lib/components";
  import { networkStore, settingsStore, walletStore } from "$lib/stores";
  import { contractDescriptors } from "$lib/contracts";
  import IconHeadingCard from "$lib/containers/Cards/IconHeadingCard.svelte";
  import { page } from "$app/stores";

  const { dashboardTransactionLimit, language } = $settingsStore;
  const { networkName } = $networkStore;

  /** @param {string} contract */
  function getIconsForContract(contract) {
    /** @type {Array.<DashboardNavItemIconProp>} */
    let icons = [{ path: "" }];

    switch (contract) {
      case "allocate":
        icons = [{ path: mdiSync }];
        break;
      case "migrate":
        icons = [{ path: mdiSwapHorizontal }];
        break;
      case "bridge":
        icons = [{ path: mdiBridge }];
        break;
      case "receive":
        icons = [{ path: mdiArrowBottomLeft }];
        break;
      case "send":
        icons = [{ path: mdiArrowTopRight }];
        break;
      case "stake":
        icons = [{ path: mdiDatabaseOutline }];
        break;
      default:
        break;
    }

    return icons;
  }

  function getExplorerPath() {
    const isStaging = $page.url.href.includes("staging");
    const baseUrl = isStaging ? "https://apps.staging" : "https://apps";

    switch (networkName) {
      case "devnet":
        return `${baseUrl}.devnet.dusk.network/explorer`;
      case "testnet":
        return `${baseUrl}.testnet.dusk.network/explorer`;
      default:
        return `${baseUrl}.dusk.network/explorer`;
    }
  }

  /** @type {ContractDescriptor[]} */
  const enabledContracts = contractDescriptors.filter(
    (contract) => contract.enabled === true
  );

  const dashboardNavItems = enabledContracts.map(({ id, label }) => ({
    href: id,
    icons: getIconsForContract(id),
    id,
    label,
  }));

  $: ({ syncStatus } = $walletStore);
</script>

{#if enabledContracts.length}
  <DashboardNav items={dashboardNavItems} />
{:else}
  <div class="no-contracts">
    <Icon path={mdiContain} size="large" />
    <h3>No Contracts Enabled</h3>
    <p>
      It appears that no contracts are currently enabled. To access the full
      range of functionalities, enabling contracts is essential.
    </p>
    {#if import.meta.env.MODE === "development"}
      <h4>For Developers:</h4>
      <p>
        No contracts are currently enabled. Please check the environment
        variables.
      </p>
    {/if}
  </div>
{/if}

<slot />

{#if import.meta.env.VITE_FEATURE_TRANSACTION_HISTORY === "true"}
  <Transactions
    items={walletStore.getTransactionsHistory()}
    {language}
    limit={dashboardTransactionLimit}
    isSyncing={syncStatus.isInProgress}
    syncError={syncStatus.error}
  />
{:else}
  <IconHeadingCard
    gap="medium"
    icons={[mdiListBoxOutline]}
    heading="Transactions"
  >
    <p>
      Transaction history will be available in an upcoming release. Meanwhile,
      you can check the status of your transactions on the Dusk Block Explorer.
    </p>

    <AnchorButton
      variant="tertiary"
      href={getExplorerPath()}
      text="Block Explorer"
      rel="noopener noreferrer"
      target="_blank"
    />
  </IconHeadingCard>
{/if}

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
