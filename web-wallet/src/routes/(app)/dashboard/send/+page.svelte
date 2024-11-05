<svelte:options immutable={true} />

<script>
  import { collect, getKey, pick } from "lamb";
  import { mdiArrowTopRight } from "@mdi/js";
  import { Send } from "$lib/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { executeSend } from "$lib/contracts";
  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { gasStore, settingsStore, walletStore } from "$lib/stores";

  /** @type {(source: "shielded" | "unshielded", balanceInfo: WalletStoreBalance) => [bigint, ContractStatus[]]}*/
  function getContractInfo(source, balanceInfo) {
    const spendable =
      source === "shielded"
        ? balanceInfo.shielded.spendable
        : balanceInfo.unshielded.value;
    const statuses = [
      {
        label: "Spendable",
        value: duskFormatter(luxToDusk(spendable)),
      },
    ];

    return [spendable, statuses];
  }

  const collectSettings = collect([
    pick(["gasLimit", "gasPrice"]),
    getKey("language"),
  ]);
  const gasLimits = $gasStore;

  /** @type {"shielded" | "unshielded"} */
  let spendableSource = "shielded";

  $: [gasSettings, language] = collectSettings($settingsStore);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
  $: ({ balance } = $walletStore);
  $: [spendable, statuses] = getContractInfo(spendableSource, balance);

  /**
   * @param {CustomEvent<{ type: "account" | "address" | undefined}>} event
   */
  function keyChangeHandler(event) {
    if (event.detail.type === "account") {
      spendableSource = "unshielded";
    } else {
      spendableSource = "shielded";
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
