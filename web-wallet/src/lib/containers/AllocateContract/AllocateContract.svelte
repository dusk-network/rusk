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
  $: currentAddress = currentProfile ? currentProfile.address.toString() : "";
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
</script>

<Allocate
  shieldedAddress={currentAddress}
  unshieldedAddress={currentAddress}
  shieldedBalance={balance.shielded.spendable}
  unshieldedBalance={balance.unshielded.value}
  execute={executeSend}
  formatter={duskFormatter}
  {gasLimits}
  {gasSettings}
  on:operationChange
/>
