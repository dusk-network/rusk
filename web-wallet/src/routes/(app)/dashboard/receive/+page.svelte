<svelte:options immutable={true} />

<script>
  import {
    mdiArrowBottomLeftThin,
    mdiShieldLock,
    mdiShieldLockOpenOutline,
  } from "@mdi/js";

  import { walletStore } from "$lib/stores";
  import { Receive } from "$lib/components";
  import { ExclusiveChoice } from "$lib/dusk/components";
  import { IconHeadingCard } from "$lib/containers/Cards";

  let addressToShow = "public";

  const options = [
    { disabled: false, label: "Public", value: "public" },
    { disabled: false, label: "Shielded", value: "shielded" },
  ];

  /** @type {"account" | "address"} */
  $: addressProp = addressToShow === "public" ? "account" : "address";
  $: ({ currentProfile } = $walletStore);
  $: currentAddress = currentProfile
    ? currentProfile[addressProp].toString()
    : "";
  $: icons = [
    mdiArrowBottomLeftThin,
    addressToShow === "public" ? mdiShieldLockOpenOutline : mdiShieldLock,
  ];
</script>

<IconHeadingCard gap="medium" heading="Receive" {icons}>
  <ExclusiveChoice {options} bind:value={addressToShow} />
  <Receive address={currentAddress} />
</IconHeadingCard>
