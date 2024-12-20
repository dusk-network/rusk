<svelte:options immutable={true} />

<script>
  import {
    mdiArrowBottomLeftThin,
    mdiShieldLock,
    mdiShieldLockOpen,
  } from "@mdi/js";

  import { walletStore } from "$lib/stores";
  import { Receive } from "$lib/components";
  import { ExclusiveChoice } from "$lib/dusk/components";
  import { IconHeadingCard } from "$lib/containers/Cards";

  let addressToShow = "shielded";

  const options = [
    { disabled: false, label: "Shielded", value: "shielded" },
    { disabled: false, label: "Public", value: "public" },
  ];

  /** @type {"address" | "account"} */
  $: addressProp = addressToShow === "shielded" ? "address" : "account";
  $: ({ currentProfile } = $walletStore);
  $: currentAddress = currentProfile
    ? currentProfile[addressProp].toString()
    : "";
  $: icons =
    import.meta.env.VITE_FEATURE_ALLOCATE === "true"
      ? [
          mdiArrowBottomLeftThin,
          addressToShow === "shielded" ? mdiShieldLock : mdiShieldLockOpen,
        ]
      : [mdiArrowBottomLeftThin];
</script>

<IconHeadingCard gap="medium" heading="Receive" {icons}>
  {#if import.meta.env.VITE_FEATURE_ALLOCATE === "true"}
    <ExclusiveChoice {options} bind:value={addressToShow} />
  {/if}

  <Receive address={currentAddress} />
</IconHeadingCard>
