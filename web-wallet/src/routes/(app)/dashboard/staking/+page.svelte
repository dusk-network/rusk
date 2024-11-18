<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { mdiDatabaseOutline } from "@mdi/js";
  import { StakeContract } from "$lib/containers";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { contractDescriptors, updateOperation } from "$lib/contracts";
  import { settingsStore, walletStore } from "$lib/stores";

  $: ({ balance } = $walletStore);

  /**
   * @param {keyof SettingsStoreContent} property
   * @param {any} value
   */
  function updateSetting(property, value) {
    settingsStore.update((store) => ({
      ...store,
      [property]: value,
    }));
  }

  onDestroy(() => {
    updateOperation("");
  });
</script>

{#if import.meta.env.VITE_FEATURE_STAKE || false}
  <IconHeadingCard
    gap="medium"
    heading="Staking"
    icons={[mdiDatabaseOutline]}
    reverse
  >
    <StakeContract
      descriptor={contractDescriptors[2]}
      spendable={balance.unshielded.value}
      on:operationChange={({ detail }) => updateOperation(detail)}
      on:suppressStakingNotice={() => updateSetting("hideStakingNotice", true)}
    />
  </IconHeadingCard>
{/if}
