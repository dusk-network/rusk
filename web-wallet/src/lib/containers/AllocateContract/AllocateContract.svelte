<svelte:options immutable={true} />

<script>
  import { collect, getKey, pick } from "lamb";
  import { createCurrencyFormatter } from "$lib/dusk/currency";
  import { gasStore, settingsStore, walletStore } from "$lib/stores";
  import { Allocate } from "$lib/components";

  const collectSettings = collect([
    pick(["gasLimit", "gasPrice"]),
    getKey("language"),
  ]);
  const gasLimits = $gasStore;

  $: [gasSettings, language] = collectSettings($settingsStore);
  $: ({ balance, currentProfile } = $walletStore);
  $: shieldedAddress = currentProfile ? currentProfile.address.toString() : "";
  $: publicAddress = currentProfile ? currentProfile.account.toString() : "";
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
</script>

<Allocate
  {shieldedAddress}
  {publicAddress}
  shieldedBalance={balance.shieldedBalance.spendable}
  publicBalance={balance.publicBalance.value}
  formatter={duskFormatter}
  {gasLimits}
  {gasSettings}
  on:operationChange
/>
