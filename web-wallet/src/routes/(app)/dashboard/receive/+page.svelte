<svelte:options immutable={true} />

<script>
  import { walletStore } from "$lib/stores";
  import { Receive } from "$lib/components";
  import { ExclusiveChoice } from "$lib/dusk/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import {
    mdiArrowBottomLeftThin,
    mdiShieldLock,
    mdiShieldLockOpen,
  } from "@mdi/js";

  $: ({ currentProfile } = $walletStore);
  $: currentAddress = currentProfile ? currentProfile.address.toString() : "";

  let addressToShow = "shielded";

  const options = [
    { disabled: false, label: "Shielded", value: "shielded" },
    { disabled: false, label: "Unshielded", value: "unshielded" },
  ];
</script>

{#if import.meta.env.VITE_FEATURE_ALLOCATE || false}
  <IconHeadingCard
    gap="medium"
    heading="Receive"
    icons={[
      mdiArrowBottomLeftThin,
      addressToShow === "shielded" ? mdiShieldLock : mdiShieldLockOpen,
    ]}
  >
    <ExclusiveChoice {options} bind:value={addressToShow} />

    <Receive address={currentAddress} />
  </IconHeadingCard>
{:else}
  <IconHeadingCard
    gap="medium"
    heading="Receive"
    icons={[mdiArrowBottomLeftThin]}
  >
    <Receive address={currentAddress} />
  </IconHeadingCard>
{/if}
