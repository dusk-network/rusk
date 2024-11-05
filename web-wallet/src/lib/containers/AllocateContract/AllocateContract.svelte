<svelte:options immutable={true} />

<script>
  import { collect, getKey, pick } from "lamb";
  import { createCurrencyFormatter } from "$lib/dusk/currency";
  import { executeSend } from "$lib/contracts";
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
  $: unshieldedAddress = currentProfile
    ? currentProfile.account.toString()
    : "";
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
</script>

<Allocate
  {shieldedAddress}
  {unshieldedAddress}
  shieldedBalance={balance.shielded.spendable}
  unshieldedBalance={balance.unshielded.value}
  execute={executeSend}
  formatter={duskFormatter}
  {gasLimits}
  {gasSettings}
  on:operationChange
/>
