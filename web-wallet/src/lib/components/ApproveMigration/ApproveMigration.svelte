<svelte:options immutable={true} />

<script>
  import { mdiAlertOutline, mdiTimerSand } from "@mdi/js";
  import { waitForTransactionReceipt } from "@wagmi/core";
  import { isHex } from "viem";
  import { createEventDispatcher } from "svelte";
  import { Button, Icon } from "$lib/dusk/components";
  import { account, wagmiConfig } from "$lib/migration/walletConnection";
  import { allowance, approve } from "$lib/migration/migration";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  /** @type {bigint} */
  export let amount;

  /** @type {HexString} */
  export let chainContract;

  /** @type {HexString} */
  export let migrationContract;

  $: ({ address } = $account);

  const dispatch = createEventDispatcher();

  const approvalTxStore = createDataStore(handleApprove);

  $: ({ isLoading, data, error } = $approvalTxStore);

  async function checkAllowance() {
    if (!address) {
      dispatch("errorApproval");
      throw new Error("Address is undefined");
    }

    try {
      const allowedAmount = await allowance(
        address,
        chainContract,
        migrationContract
      );
      return allowedAmount >= amount;
    } catch (e) {
      return false;
    }
  }

  async function handleApprove() {
    try {
      dispatch("initApproval");

      // Check initial allowance
      let isCoinApproved = await checkAllowance();

      if (isCoinApproved) {
        dispatch("incrementStep");
        return;
      }

      // Approve the transaction
      const txHash = await approve(migrationContract, chainContract, amount);

      if (!isHex(txHash)) {
        throw new Error("Transaction hash is not a valid hex string.");
      }

      // Wait for transaction confirmation
      await waitForTransactionReceipt(wagmiConfig, {
        confirmations: 3,
        hash: txHash,
      });

      // Recheck allowance after approval
      isCoinApproved = await checkAllowance();
      if (isCoinApproved) {
        dispatch("incrementStep");
      } else {
        throw new Error("Approval failed: Allowance was not updated.");
      }
    } catch {
      dispatch("errorApproval");
    }
  }
</script>

<div class="migrate__approve">
  {#if !isLoading && !error && !data}
    <div class="migrate__approve-notice">
      <p>DUSK token migration requires two transactions:</p>
      <ol class="migrate__approve-notice-list">
        <li>
          Approve: Authorize the migration contract to spend your DUSK tokens.
        </li>
        <li>Migrate: Transfer your DUSK tokens to the migration contract.</li>
      </ol>
      <p>
        Both steps must be completed for a successful migration.<br /><br
        />Warning: Make sure your wallet has enough funds to pay for the gas.
      </p>
    </div>
  {:else if isLoading}
    <div class="migrate__approve-approval">
      <Icon path={mdiTimerSand} />
      <span>Approval in progress...</span>
    </div>
  {:else if error}
    <div class="migrate__approve-approval">
      <Icon path={mdiAlertOutline} />
      <span>An error occurred during approval</span>
    </div>
  {/if}

  <Button
    text={error ? "RETRY APPROVAL" : "APPROVE MIGRATION"}
    disabled={!!isLoading}
    on:click={approvalTxStore.getData}
  />
</div>

<style lang="postcss">
  .migrate__approve {
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

      &-list {
        padding-left: 1.375em;
      }
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
