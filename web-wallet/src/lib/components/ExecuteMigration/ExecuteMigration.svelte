<svelte:options immutable={true} />

<script>
  import {
    mdiAlertOutline,
    mdiCheckDecagramOutline,
    mdiTimerSand,
  } from "@mdi/js";
  import { waitForTransactionReceipt } from "@wagmi/core";
  import { isHex } from "viem";
  import { AppAnchor, Banner } from "$lib/components";
  import { Button, Icon } from "$lib/dusk/components";
  import { account, wagmiConfig } from "$lib/migration/walletConnection";
  import { migrate } from "$lib/migration/migration";
  import { createDataStore } from "$lib/dusk/svelte-stores";
  import { createEventDispatcher } from "svelte";

  /** @type {bigint} */
  export let amount;

  /** @type {string} */
  export let currentAddress;

  /** @type {HexString} */
  export let migrationContract;

  const dispatch = createEventDispatcher();

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
      const result = await waitForTransactionReceipt(wagmiConfig, {
        confirmations: 10,
        hash: txHash,
      });
      if (result.status === "success") {
        dispatch("incrementStep");
      } else {
        throw new Error("Could not validate the transaction receipt");
      }
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
      <Icon path={mdiTimerSand} />
      <span>Migration has been submitted</span>
    </div>
    <Banner title="Migration in progress..." variant="info">
      <p>
        Migration takes some minutes to complete. Your transaction is being
        executed and you can check its status <AppAnchor
          href={`${chain.blockExplorers.default.url}/tx/${migrationHash}`}
          target="_blank"
          rel="noopener noreferrer">here</AppAnchor
        >.
      </p>
    </Banner>
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

    &-approval {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--default-gap);
      padding: 2.25em 0;
    }
  }
</style>
