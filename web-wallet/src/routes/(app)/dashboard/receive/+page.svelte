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
    { disabled: false, label: "Unshielded", value: "unshielded" },
  ];

  /** @type {"address" | "account"} */
  $: addressProp = addressToShow === "shielded" ? "address" : "account";
  $: ({ currentProfile } = $walletStore);
  $: currentAddress = currentProfile
    ? currentProfile[addressProp].toString()
    : "";
  $: icons = [
    mdiArrowBottomLeftThin,
    addressToShow === "shielded" ? mdiShieldLock : mdiShieldLockOpen,
  ];
</script>

<IconHeadingCard gap="medium" heading="Receive" {icons}>
  <ExclusiveChoice {options} bind:value={addressToShow} />
  <Receive address={currentAddress} />
</IconHeadingCard>
