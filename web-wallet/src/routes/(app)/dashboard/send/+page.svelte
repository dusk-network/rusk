<svelte:options immutable={true} />

<script>
  import { collect, getKey, pick } from "lamb";
  import { mdiArrowTopRight } from "@mdi/js";
  import { Send } from "$lib/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { executeSend } from "$lib/contracts";
  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { gasStore, settingsStore, walletStore } from "$lib/stores";

  const collectSettings = collect([
    pick(["gasLimit", "gasPrice"]),
    getKey("language"),
  ]);
  const gasLimits = $gasStore;

  /** @type {bigint} */
  let spendable = 0n;

  $: [gasSettings, language] = collectSettings($settingsStore);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
  $: ({ balance } = $walletStore);
  $: statuses = [
    {
      label: "Spendable",
      value: duskFormatter(luxToDusk(balance.shielded.spendable)),
    },
  ];
  /* eslint-disable no-sequences, no-unused-expressions */
  $: balance, (spendable = balance.shielded.spendable);
  /* eslint-enable no-sequences, no-unused-expressions */

  /**
   * @param {{type:string}} event
   */
  function keyChangeHandler(event) {
    if (event.type === "account") {
      spendable = balance.unshielded.value;
    } else {
      spendable = balance.shielded.spendable;
    }
  }
</script>

<IconHeadingCard gap="medium" heading="Send" icons={[mdiArrowTopRight]} reverse>
  <Send
    execute={executeSend}
    formatter={duskFormatter}
    {gasLimits}
    {gasSettings}
    {spendable}
    {statuses}
    enableAllocateButton={import.meta.env.VITE_FEATURE_ALLOCATE === "true"}
    enableMoonlightTransactions={import.meta.env
      .VITE_FEATURE_MOONLIGHT_TRANSACTIONS === "true"}
    on:keyChange={keyChangeHandler}
  />
</IconHeadingCard>
