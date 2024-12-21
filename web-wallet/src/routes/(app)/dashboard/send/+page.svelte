<svelte:options immutable={true} />

<script>
  import { collect, getKey, pick } from "lamb";
  import { mdiArrowTopRight } from "@mdi/js";
  import { ContractStatusesList, Send } from "$lib/components";
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
  let spendableSource = "unshielded";

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

  $: [gasSettings, language] = collectSettings($settingsStore);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
  $: ({ balance, currentProfile } = $walletStore);
  $: [spendable, statuses] = getContractInfo(spendableSource, balance);
  $: shieldedAddress = currentProfile ? currentProfile.address.toString() : "";
  $: publicAddress = currentProfile ? currentProfile.account.toString() : "";
</script>

<IconHeadingCard gap="large" heading="Send" icons={[mdiArrowTopRight]} reverse>
  <ContractStatusesList {statuses} />
  <Send
    {shieldedAddress}
    {publicAddress}
    execute={executeSend}
    formatter={duskFormatter}
    {gasLimits}
    {gasSettings}
    availableBalance={spendable}
    on:keyChange={keyChangeHandler}
  />
</IconHeadingCard>
