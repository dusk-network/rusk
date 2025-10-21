<svelte:options immutable={true} />

<script>
  import { onMount } from "svelte";
  import { collect, getKey, pick } from "lamb";
  import { getBalance, watchBlocks } from "@wagmi/core";

  import { Button } from "$lib/dusk/components";
  import { account, modal, wagmiConfig } from "$lib/web3/walletConnection";
  import { Bridge } from "$lib/components";
  import { createCurrencyFormatter } from "$lib/dusk/currency";
  import { gasStore, settingsStore, walletStore } from "$lib/stores";

  const collectSettings = collect([
    pick(["gasLimit", "gasPrice"]),
    getKey("language"),
  ]);

  /**
   * @typedef { import("@wagmi/core").GetBalanceReturnType } GetBalanceReturnType
   */
  /** @type {GetBalanceReturnType | undefined}  */
  let evmDuskBalance;

  onMount(() => {
    const unwatch = watchBlocks(wagmiConfig, {
      async onBlock() {
        if ($account.isConnected && $account.chainId && $account.address) {
          try {
            evmDuskBalance = await getBalance(wagmiConfig, {
              address: $account.address,
              chainId: $account.chainId,
            });
          } catch (e) {
            // eslint-disable-next-line no-console
            console.error("getBalance failed", e);
            evmDuskBalance = undefined;
          }
        }
      },
    });

    return () => {
      unwatch();
    };
  });

  $: ({ isConnected } = $account);
  $: ({ balance, currentProfile } = $walletStore);
  $: [gasSettings, language] = collectSettings($settingsStore);
  $: gasLimits = $gasStore;
  $: unshieldedAddress = currentProfile
    ? currentProfile.account.toString()
    : "";
  $: formatter = createCurrencyFormatter(language, "DUSK", 9);
</script>

{#if !isConnected}
  <Button text="CONNECT WEB3 WALLET" on:click={() => modal.open()} />
{:else}
  <Bridge
    {unshieldedAddress}
    {evmDuskBalance}
    unshieldedBalance={balance.unshielded.value}
    {formatter}
    {gasLimits}
    {gasSettings}
    on:operationChange
  />
{/if}
