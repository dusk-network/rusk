<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { mdiDatabaseOutline } from "@mdi/js";
  import { StakeContract } from "$lib/containers";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { contractDescriptors, updateOperation } from "$lib/contracts";
  import { settingsStore } from "$lib/stores";

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

{#if !!import.meta.env.VITE_CONTRACT_STAKE_DISABLED}
  <IconHeadingCard
    gap="medium"
    heading="Staking"
    iconPath={mdiDatabaseOutline}
    reverse
  >
    <StakeContract
      descriptor={contractDescriptors[1]}
      on:operationChange={({ detail }) => updateOperation(detail)}
      on:suppressStakingNotice={() => updateSetting("hideStakingNotice", true)}
    />
  </IconHeadingCard>
{/if}
