<svelte:options immutable={true} />

<script>
  import { collect, getKey, pick } from "lamb";
  import { mdiArrowTopRight } from "@mdi/js";
  import { Send } from "$lib/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { executeSend } from "$lib/contracts";
  import { createCurrencyFormatter } from "$lib/dusk/currency";
  import { gasStore, settingsStore, walletStore } from "$lib/stores";

  const collectSettings = collect([
    pick(["gasLimit", "gasPrice"]),
    getKey("language"),
  ]);
  const gasLimits = $gasStore;

  $: [gasSettings, language] = collectSettings($settingsStore);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
  $: ({ balance } = $walletStore);
  $: statuses = [
    {
      label: "Spendable",
      value: duskFormatter(balance.maximum),
    },
  ];
</script>

<IconHeadingCard gap="medium" heading="Send" icons={[mdiArrowTopRight]} reverse>
  <Send
    execute={executeSend}
    formatter={duskFormatter}
    {gasLimits}
    {gasSettings}
    spendable={balance.maximum}
    {statuses}
    disableAllocateButton={import.meta.env.VITE_CONTRACT_ALLOCATE_DISABLED ===
      "true"}
  />
</IconHeadingCard>
