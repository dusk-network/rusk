<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { filterWith, find, hasKeyValue, last } from "lamb";
  import { mdiDatabaseOutline, mdiSwapVertical } from "@mdi/js";

  import { Tabs } from "$lib/dusk/components";
  import { StakeContract, TransferContract } from "$lib/containers";
  import { AddressPicker, Balance, Transactions } from "$lib/components";
  import { operationsStore, settingsStore, walletStore } from "$lib/stores";
  import { contractDescriptors } from "$lib/contracts";

  /** @type {import('./$types').PageData} */
  export let data;

  /** @type {number | undefined} */
  let fiatPrice;

  const { currency, dashboardTransactionLimit, language } = $settingsStore;

  data.currentPrice.then((prices) => {
    fiatPrice = prices[currency.toLowerCase()];
  });

  /** @type {(descriptors: ContractDescriptor[]) => ContractDescriptor[]} */
  const getEnabledContracts = filterWith(hasKeyValue("disabled", false));

  /** @param {string} id */
  function updateOperation(id) {
    operationsStore.update((store) => ({
      ...store,
      currentOperation: id,
    }));
  }

  /**
   * @param {keyof import("$lib/stores/stores").SettingsStore} property
   * @param {any} value
   */
  function updateSetting(property, value) {
    settingsStore.update((store) => ({
      ...store,
      [property]: value,
    }));
  }

  const enabledContracts = getEnabledContracts(contractDescriptors);
  const tabItems = enabledContracts.map(({ id, label }) => ({
    icon: { path: id === "transfer" ? mdiSwapVertical : mdiDatabaseOutline },
    id,
    label,
  }));

  let selectedTab = tabItems[0]?.id ?? "";

  $: selectedContract = find(enabledContracts, hasKeyValue("id", selectedTab));
  $: ({ balance, currentAddress, addresses } = $walletStore);
  $: ({ currentOperation } = $operationsStore);

  onDestroy(() => {
    updateOperation("");
  });
</script>

<div class="dashboard-content">
  <h2 class="visible-hidden">Dashboard</h2>

  <AddressPicker {addresses} {currentAddress} />

  <Balance
    fiatCurrency={currency}
    {fiatPrice}
    locale={language}
    tokenCurrency="DUSK"
    tokens={balance.value}
  />

  {#if selectedContract}
    <article class="tabs">
      <Tabs
        bind:selectedTab
        items={tabItems}
        on:change={() => updateOperation("")}
      />
      <div
        class="tabs__panel"
        class:tabs__panel--first={selectedTab === enabledContracts[0].id}
        class:tabs__panel--last={selectedTab === last(enabledContracts).id}
      >
        {#key selectedTab}
          <div in:fade class="tabs__contract">
            <svelte:component
              this={selectedTab === "transfer"
                ? TransferContract
                : StakeContract}
              descriptor={selectedContract}
              on:suppressStakingNotice={() =>
                updateSetting("hideStakingNotice", true)}
              on:operationChange={({ detail }) => updateOperation(detail)}
            />
          </div>
        {/key}
      </div>
    </article>
  {/if}

  {#if currentOperation === "" && selectedTab === "transfer"}
    <Transactions
      items={walletStore.getTransactionsHistory()}
      {language}
      limit={dashboardTransactionLimit}
    />
  {/if}
</div>

<style lang="postcss">
  .dashboard-content {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1.375rem;
    overflow-y: auto;
    flex: 1;
  }

  .tabs {
    &__panel {
      border-radius: var(--control-border-radius-size);
      background: var(--surface-color);
      transition: border-radius 0.4s ease-in-out;

      &--first {
        border-top-left-radius: 0;
      }

      &--last {
        border-top-right-radius: 0;
      }
    }

    &__contract {
      display: flex;
      flex-direction: column;
      padding: 1rem 1.375rem;
      gap: var(--default-gap);
    }
  }
</style>
