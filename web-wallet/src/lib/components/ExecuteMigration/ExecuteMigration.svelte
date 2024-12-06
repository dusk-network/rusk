<svelte:options immutable={true} />

<script>
  import {
    mdiAlertOutline,
    mdiCheckDecagramOutline,
    mdiTimerSand,
  } from "@mdi/js";
  import { waitForTransactionReceipt } from "@wagmi/core";
  import { isHex } from "viem";
  import { AppAnchor } from "$lib/components";
  import { Button, Icon } from "$lib/dusk/components";
  import { account, wagmiConfig } from "$lib/migration/walletConnection";
  import { migrate } from "$lib/migration/migration";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  /** @type {bigint} */
  export let amount;

  /** @type {string} */
  export let currentAddress;

  /** @type {HexString} */
  export let migrationContract;

  /** @type {string} */
  let migrationHash = "";

  const migrationStore = createDataStore(handleMigration);

  $: ({ chain, chainId } = $account);
  $: ({ data, error, isLoading } = $migrationStore);

  /** @param {number} id - the chain id of the selected smart contract */
  async function handleMigration(id) {
    const txHash = await migrate(amount, id, currentAddress, migrationContract);

    if (isHex(txHash)) {
      migrationHash = txHash;
      return waitForTransactionReceipt(wagmiConfig, {
        confirmations: 10,
        hash: txHash,
      });
    } else {
      throw new Error("txHash is not a hex string");
    }
  }
</script>

<div class="migrate__execute">
  {#if !isLoading && !data && !error}
    <div class="migrate__execute-approval">
      <Icon path={mdiCheckDecagramOutline} />
      <span>Migration Approved</span>
    </div>
  {:else if error}
    <div class="migrate__execute-approval">
      <Icon path={mdiAlertOutline} />
      <span>Action has been rejected on the connected wallet.</span>
    </div>
  {:else if isLoading && !migrationHash}
    <div class="migrate__execute-approval">
      <Icon path={mdiTimerSand} />
      <span>Migration in progress...</span>
    </div>
  {/if}
  {#if migrationHash && chain?.blockExplorers}
    <div class="migrate__execute-approval">
      <Icon path={mdiCheckDecagramOutline} />
      <span>Migration has been submitted</span>
    </div>
    <div class="migrate__execute-notice">
      <span
        >Migration takes some minutes to complete. Your transaction is being
        executed and you can check it <AppAnchor
          href={`${chain.blockExplorers.default.url}/tx/${migrationHash}`}
          target="_blank"
          rel="noopener noreferrer">here</AppAnchor
        >.</span
      >
    </div>
  {/if}
  {#if (isLoading || !data || error) && !migrationHash}
    <Button
      text={`${error ? "RETRY" : "EXECUTE"}  MIGRATION`}
      disabled={!!isLoading}
      on:click={() => migrationStore.getData(chainId)}
    />
  {/if}
</div>

<style lang="postcss">
  .migrate__execute {
    display: flex;
    justify-content: center;
    flex-direction: column;

    &-notice {
      font-size: 0.875em;
      line-height: 1.3125em;
      padding: 1em 1.375em;
      border-radius: 0.675em;
      border: 1px solid var(--primary-color);
      margin-top: 0.625em;
      margin-bottom: 1em;
    }

    &-approval {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--default-gap);
      padding: 2.25em 0;
    }
  }
</style>
