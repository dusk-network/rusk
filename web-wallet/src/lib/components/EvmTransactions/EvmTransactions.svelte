<script>
  import { mdiArrowLeft, mdiContain } from "@mdi/js";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";

  import { goto } from "$lib/navigation";

  import { formatBlocksAsTime } from "$lib/bridge/formatBlocksAsTime";
  import { AppAnchorButton } from "$lib/components";
  import { Button, Icon, Suspense, Throbber } from "$lib/dusk/components";
  import { createTransferFormatter, luxToDusk } from "$lib/dusk/currency";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { networkStore, walletStore } from "$lib/stores";
  import wasmPath from "$lib/vendor/standard_bridge_dd_opt.wasm?url";

  /** @type {string} */
  const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;

  /** @type {string} */
  export let language;

  /** @type {Promise<PendingWithdrawalEntry[]>} */
  export let items = Promise.resolve(
    /** @type {PendingWithdrawalEntry[]} */ ([])
  );

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {number} */
  let ellipsisChars = calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20);

  /** @type {(n: bigint|number) => string} */
  let transferFormatter;

  /** @type {Intl.NumberFormat} */
  let numberFormatter;

  /** @type {{ height: bigint; period: bigint } | null} */
  let chainInfo = null;

  async function loadChainInfo() {
    const [height, periodNum] =
      await getCurrentBlockHeightAndFinalizationPeriod();
    chainInfo = { height, period: BigInt(periodNum) };
  }

  /**
   * @param {number} txHeight
   * @returns {bigint} remaining blocks (>= 0n)
   */
  function remainingBlocks(txHeight) {
    if (!chainInfo) return 0n;

    const height = BigInt(txHeight);
    const remaining = height + chainInfo.period - chainInfo.height;

    return remaining > 0n ? remaining : 0n;
  }

  /**
   * @return {Promise<number>}
   */
  async function getFinalizationPeriod() {
    const contract = await walletStore.useContract(
      VITE_BRIDGE_CONTRACT_ID,
      wasmPath
    );
    return await contract.call.finalization_period();
  }

  /**
   * @return {Promise<bigint>}
   */
  async function getCurrentBlockHeight() {
    return await networkStore.getCurrentBlockHeight();
  }

  /**
   * @return {Promise<[bigint, number]>}
   */
  async function getCurrentBlockHeightAndFinalizationPeriod() {
    return await Promise.all([
      getCurrentBlockHeight(),
      getFinalizationPeriod(),
    ]);
  }

  onMount(() => {
    loadChainInfo();

    function onResize() {
      screenWidth = window.innerWidth;
    }

    window.addEventListener("resize", onResize);

    return () => {
      window.removeEventListener("resize", onResize);
    };
  });

  $: ({ currentProfile } = $walletStore);
  $: ellipsisChars = calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20);
  $: transferFormatter = createTransferFormatter(language);
  $: numberFormatter = new Intl.NumberFormat(language);
  $: currentProfileAccountAddress = currentProfile
    ? currentProfile.account.toString()
    : "";
</script>

<article in:fade|global class="transactions">
  <header class="transactions__header">
    <h3 class="h4">Pending Withdrawals</h3>
    <AppAnchorButton
      className="transactions__footer-button"
      href="/dashboard/bridge"
      text="Back"
      variant="tertiary"
      icon={{ path: mdiArrowLeft }}
    />
  </header>
  <Suspense
    className="transactions-list__container"
    errorMessage="Error getting transactions"
    errorVariant="details"
    waitFor={items}
  >
    <svelte:fragment slot="pending-content">
      <div class="transactions-list__loading-container">
        <Throbber />
      </div>
    </svelte:fragment>
    <svelte:fragment slot="success-content" let:result={transactions}>
      {@const userTransactions =
        currentProfileAccountAddress && Array.isArray(transactions)
          ? transactions.filter(
              ([, tx]) => tx?.to?.External === currentProfileAccountAddress
            )
          : []}

      {#if userTransactions.length}
        {#each userTransactions as [txId, tx] (txId)}
          {@const amount = BigInt(tx.amount)}
          <dl class="transactions-list">
            <dt class="transactions-list__term">Block</dt>
            <dd class="transactions-list__datum">
              {numberFormatter.format(tx.block_height)}
            </dd>
            <dt class="transactions-list__term">Amount</dt>
            <dd class="transactions-list__datum">
              {transferFormatter(luxToDusk(amount))}
              <span class="transactions-list__ticker">Dusk</span>
            </dd>
            <dt class="transactions-list__term">From</dt>
            <dd class="transactions-list__datum">
              {middleEllipsis(tx.from, ellipsisChars)}
            </dd>
            {#if chainInfo}
              {#if chainInfo.height >= BigInt(tx.block_height) + chainInfo.period}
                <dt class="transactions-list__term">Status</dt>
                <dd class="transactions-list__datum">
                  <Button
                    text="Finalize now"
                    on:click={async () => {
                      try {
                        const res =
                          await walletStore.finalizeWithdrawalEvmFunctionCall(
                            VITE_BRIDGE_CONTRACT_ID,
                            txId,
                            wasmPath
                          );
                        const hash = res.hash;
                        await goto("/dashboard/bridge/transactions/complete", {
                          replaceState: true,
                          state: {
                            hash,
                          },
                        });
                      } catch (e) {
                        // eslint-disable-next-line no-console
                        console.error("Finalize failed", e);
                      }
                    }}
                  />
                </dd>
              {:else}
                {@const remBlocks = remainingBlocks(tx.block_height)}
                <dt class="transactions-list__term">Status</dt>
                <dd class="transactions-list__datum">
                  Finalization possible in {numberFormatter.format(remBlocks)} blocks
                  ({formatBlocksAsTime(remBlocks, language)} at ~10s/block)
                </dd>
              {/if}
            {:else}
              <dt class="transactions-list__term">Status</dt>
              <dd class="transactions-list__datum"><Throbber /></dd>
            {/if}
          </dl>
        {/each}
      {:else}
        <div class="transactions-list__empty">
          <Icon path={mdiContain} size="large" />
          <p>
            You have no pending withdrawals. If you have just made a withdrawal,
            it can take up to 10 minutes to appear here (depending on network
            usage).
          </p>
        </div>
      {/if}
    </svelte:fragment>
  </Suspense>
</article>

<style lang="postcss">
  .transactions {
    border-radius: 1.25em;
    background: var(--surface-color);
    display: flex;
    flex-direction: column;
    gap: var(--default-gap);
    padding-top: 1.375em;

    &__header {
      display: flex;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      padding: 0 1rem;
      gap: 0.625rem;
      flex-wrap: wrap;

      & :global(h3) {
        line-height: 150%;
      }
    }
  }

  :global {
    .transactions-list {
      display: grid;
      grid-template-columns: max-content 1fr;
      width: 100%;

      &__term {
        background-color: var(--background-color-alt);
        grid-column: 1;
        line-height: 130%;
        text-transform: capitalize;
        padding: 0.3125em 0.625em 0.3125em 1.375em;
      }

      &__ticker {
        text-transform: uppercase;
      }

      &__loading-container {
        margin: 1.25em 0;
      }

      &__datum {
        grid-column: 2;
        line-height: 150%;
        padding: 0.312em 0.625em;
        display: flex;
        align-items: center;
        gap: 0.625em;
        font-family: var(--mono-font-family);
        overflow: hidden;

        & samp {
          display: block;
          white-space: nowrap;
          overflow: hidden;
        }

        &--hash {
          justify-content: center;
        }
      }

      &__empty {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.5em;
        margin: 1.25em 0;
      }

      &__badge {
        flex: 1;
      }

      & dt:first-of-type,
      & dd:first-of-type {
        padding-top: 1em;
      }

      & dt:last-of-type,
      & dd:last-of-type {
        padding-bottom: 1em;
      }

      & dt:first-of-type {
        border-top-right-radius: 2em;
      }

      & dt:last-of-type {
        border-bottom-right-radius: 2em;
      }
    }
  }
</style>
